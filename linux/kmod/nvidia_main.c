// SPDX-License-Identifier: MIT
/*
 * Hermes GSP primary kernel surface exported as module name "nvidia".
 * Init runs the shared evidence-driven bring-up.  Online is published only
 * after the hardware session supplies every required token.
 */

#include <linux/module.h>
#include <linux/pci.h>
#include <linux/firmware.h>
#include <linux/iommu.h>
#include <linux/slab.h>
#include <crypto/hash.h>

#include "include/hermes_kmod.h"

static bool hermes_online;
static enum hermes_phase hermes_phase = HERMES_PHASE_OFFLINE;

/*
 * allow_sim_promote=1 enables HERMES_CTL_IOCTL_SIM_PROMOTE (complete-evidence
 * Online for integration tests). Default 0: simulation cannot mint hardware
 * evidence.
 */
bool hermes_allow_sim_promote;
module_param_named(allow_sim_promote, hermes_allow_sim_promote, bool, 0644);
MODULE_PARM_DESC(allow_sim_promote,
		 "Allow SIM_PROMOTE ioctl (complete-evidence Online; not silicon)");
EXPORT_SYMBOL_GPL(hermes_allow_sim_promote);

/* Firmware is staged by the operator; the digest pins remain in Hermes. */
static char *hermes_firmware_version = "610.57.04";
module_param_named(firmware_version, hermes_firmware_version, charp, 0644);
MODULE_PARM_DESC(firmware_version,
		 "OpenRM GSP firmware directory under nvidia/ (digest-pinned)");

static int hermes_sha256(const u8 *data, size_t length, u8 digest[32])
{
	struct crypto_shash *tfm;
	struct shash_desc *desc;
	int err;

	if (!data || !digest)
		return -EINVAL;
	if (length > UINT_MAX)
		return -EFBIG;

	tfm = crypto_alloc_shash("sha256", 0, 0);
	if (IS_ERR(tfm))
		return PTR_ERR(tfm);
	desc = kzalloc(sizeof(*desc) + crypto_shash_descsize(tfm), GFP_KERNEL);
	if (!desc) {
		crypto_free_shash(tfm);
		return -ENOMEM;
	}
	desc->tfm = tfm;
	err = crypto_shash_init(desc);
	if (!err)
		err = crypto_shash_update(desc, data, (unsigned int)length);
	if (!err)
		err = crypto_shash_final(desc, digest);
	kfree(desc);
	crypto_free_shash(tfm);
	return err;
}

static int hermes_measure_staged_firmware(struct pci_dev *pdev, u16 device)
{
	const struct firmware *fw = NULL;
	u8 digest[32];
	char path[128];
	const char *family;
	int err;

	if (!hermes_firmware_version || !*hermes_firmware_version)
		return -EINVAL;
	/* Keep this mapping identical to hermes-gsp::firmware_family_for_device. */
	if ((device >= 0x1e00 && device <= 0x1fff) ||
	    (device >= 0x2000 && device <= 0x20ff) ||
	    (device >= 0x2180 && device <= 0x21ff))
		family = "tu10x";
	else
		family = "ga10x";
	snprintf(path, sizeof(path), "nvidia/%s/gsp_%s.bin",
		 hermes_firmware_version, family);
	err = request_firmware(&fw, path, &pdev->dev);
	if (err) {
		dev_warn(&pdev->dev,
			 "Hermes GSP firmware %s unavailable (%d); device remains offline\n",
			 path, err);
		return err;
	}
	if (fw->size > U32_MAX) {
		err = -EFBIG;
		dev_warn(&pdev->dev, "Hermes GSP firmware %s is too large\n", path);
		goto out_release;
	}
	err = hermes_sha256(fw->data, fw->size, digest);
	if (err) {
		dev_warn(&pdev->dev, "Hermes GSP firmware %s hash failed (%d)\n",
			 path, err);
		goto out_release;
	}
	err = hermes_firmware_measure((u32)fw->size, digest);
	if (err)
		dev_warn(&pdev->dev,
			 "Hermes GSP firmware %s rejected by the digest pins (%d)\n",
			 path, err);
	else
		dev_info(&pdev->dev,
			 "Hermes GSP firmware %s measured and admitted (len=%zu)\n",
			 path, fw->size);

out_release:
	release_firmware(fw);
	return err;
}

struct hermes_iommu_group_count {
	unsigned int devices;
};

static int hermes_count_iommu_group_devices(struct device *dev, void *data)
{
	struct hermes_iommu_group_count *count = data;

	if (dev && count)
		count->devices++;
	return 0;
}

/*
 * Collect only evidence that the kernel can actually prove.  A translated
 * domain plus a singleton IOMMU group gives Hermes an isolated DMA owner; a
 * missing group, a shared group, or a passthrough domain is not promoted.
 */
static bool hermes_collect_iommu_evidence(struct pci_dev *pdev, u32 *domain_id)
{
	struct hermes_iommu_group_count count = { 0 };
	struct iommu_group *group;
	struct iommu_domain *domain;
	int group_id;
	int err;
	bool isolated;

	if (!pdev || !domain_id)
		return false;
	group = iommu_group_get(&pdev->dev);
	if (!group)
		return false;
	domain = iommu_get_domain_for_dev(&pdev->dev);
	group_id = iommu_group_id(group);
	err = iommu_group_for_each_dev(group, &count,
				       hermes_count_iommu_group_devices);
	isolated = !err && domain && group_id > 0 && count.devices == 1;
	if (isolated)
		*domain_id = (u32)group_id;
	iommu_group_put(group);
	return isolated;
}

bool hermes_gsp_is_online(void)
{
	return hermes_online;
}
EXPORT_SYMBOL_GPL(hermes_gsp_is_online);

enum hermes_phase hermes_gsp_phase(void)
{
	return hermes_phase;
}
EXPORT_SYMBOL_GPL(hermes_gsp_phase);

void hermes_gsp_set_state(bool online, enum hermes_phase phase)
{
	hermes_online = online;
	hermes_phase = phase;
}
EXPORT_SYMBOL_GPL(hermes_gsp_set_state);

static int hermes_try_bind_device(struct pci_dev *pdev)
{
	struct hermes_pci_id id = {
		.vendor = pdev->vendor,
		.device = pdev->device,
		.class_code = (pdev->class >> 16) & 0xff,
		.subclass = (pdev->class >> 8) & 0xff,
	};
	/*
	 * Without staged firmware + IOMMU domain from a live path, evidence
	 * stays incomplete. Module load must not claim Online.
	 */
	struct hermes_hw_evidence ev = {
		.iommu_isolated = false,
		.dma_domain = 0,
		.wpr_locked = false,
		.mailbox_ok = false,
		.ready_ok = false,
		.firmware_measured = false,
	};
	struct hermes_bringup_result r;
	int firmware_err;

	if (id.vendor != HERMES_VENDOR_NVIDIA)
		return -ENODEV;
	if (!hermes_is_turing_or_newer(id.device)) {
		pr_info("hermes/nvidia: skip pre-Turing device %04x:%04x\n",
			id.vendor, id.device);
		return -ENODEV;
	}

	/* Load and hash the staged OpenRM image during the real PCI probe. */
	firmware_err = hermes_measure_staged_firmware(pdev, id.device);
	ev.firmware_measured = (firmware_err == 0);
	ev.iommu_isolated = hermes_collect_iommu_evidence(pdev, &ev.dma_domain);
	if (ev.iommu_isolated)
		pr_info("hermes/nvidia: isolated IOMMU group domain=%u\n", ev.dma_domain);
	else
		pr_info("hermes/nvidia: IOMMU isolation evidence unavailable\n");

	r = hermes_run_bringup(&id, &ev);
	hermes_gsp_set_state(r.online, r.phase);
	pr_info("hermes/nvidia: bring-up status=%d phase=%s online=%d firmware=%s (device %04x:%04x)\n",
		r.status, hermes_phase_name(r.phase), r.online,
		ev.firmware_measured ? "admitted" : "unavailable/rejected", id.vendor,
		id.device);
	return 0;
}

static int hermes_pci_probe(struct pci_dev *pdev, const struct pci_device_id *ent)
{
	int err;

	err = pcim_enable_device(pdev);
	if (err)
		return err;
	return hermes_try_bind_device(pdev);
}

static void hermes_pci_remove(struct pci_dev *pdev)
{
	hermes_gsp_set_state(false, HERMES_PHASE_OFFLINE);
	pr_info("hermes/nvidia: unbound %s\n", pci_name(pdev));
}

static const struct pci_device_id hermes_pci_table[] = {
	{
		.vendor = PCI_VENDOR_ID_NVIDIA,
		.device = PCI_ANY_ID,
		.subvendor = PCI_ANY_ID,
		.subdevice = PCI_ANY_ID,
		.class = PCI_CLASS_DISPLAY_VGA << 8,
		.class_mask = 0xffff00,
	},
	{
		.vendor = PCI_VENDOR_ID_NVIDIA,
		.device = PCI_ANY_ID,
		.subvendor = PCI_ANY_ID,
		.subdevice = PCI_ANY_ID,
		.class = PCI_CLASS_DISPLAY_3D << 8,
		.class_mask = 0xffff00,
	},
	{
		.vendor = PCI_VENDOR_ID_ATI,
		.device = PCI_ANY_ID,
		.subvendor = PCI_ANY_ID,
		.subdevice = PCI_ANY_ID,
		.class = PCI_CLASS_DISPLAY_VGA << 8,
		.class_mask = 0xffff00,
	},
	{
		.vendor = PCI_VENDOR_ID_INTEL,
		.device = PCI_ANY_ID,
		.subvendor = PCI_ANY_ID,
		.subdevice = PCI_ANY_ID,
		.class = PCI_CLASS_DISPLAY_VGA << 8,
		.class_mask = 0xffff00,
	},
	{ 0 }
};
MODULE_DEVICE_TABLE(pci, hermes_pci_table);

static struct pci_driver hermes_pci_driver = {
	.name = "nvidia",
	.id_table = hermes_pci_table,
	.probe = hermes_pci_probe,
	.remove = hermes_pci_remove,
};

static int __init hermes_nvidia_init(void)
{
	int err;

	pr_info("hermes/nvidia: loading Hermes GSP surface (evidence-driven)\n");
	hermes_gsp_set_state(false, HERMES_PHASE_OFFLINE);
	err = hermes_chardev_init();
	if (err) {
		pr_err("hermes/nvidia: chardev_init failed: %d\n", err);
		return err;
	}
	err = pci_register_driver(&hermes_pci_driver);
	if (err) {
		pr_err("hermes/nvidia: pci_register_driver failed: %d\n", err);
		hermes_chardev_exit();
		return err;
	}
	pr_info("hermes/nvidia: registered; phase=%s online=%d\n",
		hermes_phase_name(hermes_gsp_phase()), hermes_gsp_is_online());
	return 0;
}

static void __exit hermes_nvidia_exit(void)
{
	pci_unregister_driver(&hermes_pci_driver);
	hermes_chardev_exit();
	hermes_gsp_set_state(false, HERMES_PHASE_OFFLINE);
	pr_info("hermes/nvidia: unloaded\n");
}

module_init(hermes_nvidia_init);
module_exit(hermes_nvidia_exit);

MODULE_DESCRIPTION("Hermes GSP drop-in surface (module name: nvidia)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_VERSION("0.1.0");
MODULE_FIRMWARE("nvidia/610.57.04/gsp_tu10x.bin");
MODULE_FIRMWARE("nvidia/610.57.04/gsp_ga10x.bin");
