// SPDX-License-Identifier: MIT
/*
 * Hermes GSP primary kernel surface exported as module name "nvidia".
 * Init runs the shared fail-closed bring-up; Online is never invented.
 */

#include <linux/module.h>
#include <linux/pci.h>

#include "include/hermes_kmod.h"

static bool hermes_online;
static enum hermes_phase hermes_phase = HERMES_PHASE_OFFLINE;

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

	if (id.vendor != HERMES_VENDOR_NVIDIA)
		return -ENODEV;
	if (!hermes_is_turing_or_newer(id.device)) {
		pr_info("hermes/nvidia: skip pre-Turing device %04x:%04x\n",
			id.vendor, id.device);
		return -ENODEV;
	}

	r = hermes_run_bringup(&id, &ev);
	hermes_phase = r.phase;
	hermes_online = r.online;
	pr_info("hermes/nvidia: bring-up status=%d phase=%s online=%d (device %04x:%04x)\n",
		r.status, hermes_phase_name(r.phase), r.online, id.vendor,
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
	hermes_online = false;
	hermes_phase = HERMES_PHASE_OFFLINE;
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

	pr_info("hermes/nvidia: loading Hermes GSP surface (fail-closed)\n");
	hermes_online = false;
	hermes_phase = HERMES_PHASE_OFFLINE;
	err = pci_register_driver(&hermes_pci_driver);
	if (err) {
		pr_err("hermes/nvidia: pci_register_driver failed: %d\n", err);
		return err;
	}
	pr_info("hermes/nvidia: registered; phase=%s online=%d\n",
		hermes_phase_name(hermes_phase), hermes_online);
	return 0;
}

static void __exit hermes_nvidia_exit(void)
{
	pci_unregister_driver(&hermes_pci_driver);
	hermes_online = false;
	hermes_phase = HERMES_PHASE_OFFLINE;
	pr_info("hermes/nvidia: unloaded\n");
}

module_init(hermes_nvidia_init);
module_exit(hermes_nvidia_exit);

MODULE_DESCRIPTION("Hermes GSP drop-in surface (module name: nvidia)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_VERSION("0.1.0");
