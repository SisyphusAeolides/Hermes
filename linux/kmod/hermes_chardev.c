// SPDX-License-Identifier: MIT
/*
 * Classic NVIDIA character device names: /dev/nvidiactl, /dev/nvidia0.
 * Open/ioctl remain available for status while GSP is Offline. Device
 * operations are evidence-gated and never synthesize an Online session.
 */

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/cdev.h>
#include <linux/device.h>
#include <linux/uaccess.h>
#include <linux/mutex.h>
#include <linux/version.h>
#include <linux/namei.h>
#include <linux/path.h>
#include <linux/pci.h>
#include <linux/string.h>

#include "include/hermes_kmod.h"
#include "include/hermes_ctl_uapi.h"

#define HERMES_CHAR_NAME_CTL "nvidiactl"
#define HERMES_CHAR_NAME_0 "nvidia0"
#define HERMES_CHAR_COUNT 2

extern bool hermes_gsp_is_online(void);
extern enum hermes_phase hermes_gsp_phase(void);
extern void hermes_gsp_set_state(bool online, enum hermes_phase phase);
extern bool hermes_allow_sim_promote;

static dev_t hermes_char_devt;
static struct class *hermes_char_class;
static struct cdev hermes_char_cdev;
static DEFINE_MUTEX(hermes_char_lock);

/* Sticky evidence from MEASURE_FW / APPLY_EVIDENCE until the session is complete. */
static bool hermes_fw_admitted;
static struct hermes_hw_evidence hermes_sticky_ev = {
	.iommu_isolated = false,
	.dma_domain = 0,
	.wpr_locked = false,
	.mailbox_ok = false,
	.ready_ok = false,
	.firmware_measured = false,
};

/* Embedded OpenRM GSP-RM pins (match hermes-gsp firmware.rs allow-list). */
static const struct hermes_fw_pin hermes_fw_allow[] = {
	/* 610.43.02 tu10x */
	{ .byte_length = 29352832,
	  .sha256 = { 0xc8, 0xfc, 0x1a, 0x92, 0xc9, 0x0b, 0x03, 0x4b, 0xbb, 0xe4,
		      0xd5, 0x6c, 0xa9, 0x4b, 0x0d, 0xc9, 0x5a, 0xfb, 0x52, 0xd3,
		      0x40, 0x9a, 0x78, 0x80, 0x18, 0x6a, 0xe0, 0x3c, 0x7d, 0xde,
		      0x17, 0xf3 } },
	/* 610.43.02 ga10x */
	{ .byte_length = 84277400,
	  .sha256 = { 0x00, 0xda, 0x3f, 0xd9, 0xb4, 0x1d, 0xb8, 0xaf, 0xd6, 0x61,
		      0xc9, 0xdc, 0xec, 0x2a, 0x32, 0xa3, 0x1d, 0x3c, 0x14, 0xb9,
		      0x3e, 0x6d, 0x71, 0x12, 0xd4, 0xfb, 0x3f, 0x46, 0x87, 0x65,
		      0x25, 0xce } },
	/* 610.43.03 tu10x */
	{ .byte_length = 29352832,
	  .sha256 = { 0x73, 0x06, 0x56, 0x19, 0xdb, 0x9e, 0xc9, 0x21, 0xd1, 0x9f,
		      0xc4, 0xe5, 0x19, 0xdd, 0x04, 0xd9, 0x1a, 0x91, 0x99, 0xb5,
		      0x25, 0xea, 0xca, 0x9b, 0x25, 0x7b, 0x89, 0xfb, 0x8c, 0x5e,
		      0x52, 0xc0 } },
	/* 610.43.03 ga10x */
	{ .byte_length = 84277400,
	  .sha256 = { 0x57, 0x23, 0x73, 0x62, 0x0a, 0x37, 0x41, 0x8f, 0x24, 0xdc,
		      0x16, 0xb5, 0x03, 0x1c, 0x39, 0x33, 0x87, 0x78, 0xc3, 0x25,
		      0x7e, 0x48, 0xe8, 0x40, 0x8d, 0xe9, 0xa5, 0x72, 0x91, 0xb2,
		      0x4f, 0x3a } },
	/* 610.57.04 tu10x */
	{ .byte_length = 29381504,
	  .sha256 = { 0xd1, 0x57, 0xe3, 0xb7, 0xdd, 0x5d, 0xa2, 0xca, 0x8d, 0x1c,
		      0xcb, 0x6c, 0xa9, 0x89, 0x58, 0xf9, 0xe3, 0x5d, 0x10, 0xa9,
		      0xef, 0x73, 0x26, 0x27, 0x7e, 0xba, 0xc1, 0x33, 0xe4, 0xb0,
		      0xd1, 0xa7 } },
	/* 610.57.04 ga10x */
	{ .byte_length = 84310168,
	  .sha256 = { 0xc0, 0x15, 0x69, 0x54, 0xf3, 0xe0, 0x48, 0xd5, 0x60, 0x11,
		      0x52, 0x4e, 0x0c, 0x2a, 0xe2, 0x88, 0x1b, 0xb6, 0xdb, 0x81,
		      0x73, 0xb5, 0x3f, 0x9b, 0x2f, 0x4e, 0xb9, 0x41, 0x97, 0xf0,
	 0x29, 0x99 } },
};

/*
 * Match and remember a firmware measurement made by the in-kernel loader.
 * Keeping this boundary in the primary module means a probe cannot advance
 * the GSP session merely because a firmware file exists: the complete byte
 * length and SHA-256 digest must match an embedded OpenRM pin first.
 */
static int __hermes_firmware_measure(u32 byte_length, const u8 *sha256)
{
	int admitted;

	if (!sha256)
		return -EINVAL;
	admitted = hermes_fw_pin_match(hermes_fw_allow,
					       ARRAY_SIZE(hermes_fw_allow),
					       byte_length, sha256);
	hermes_fw_admitted = admitted ? true : false;
	hermes_sticky_ev.firmware_measured = admitted ? true : false;
	if (!admitted)
		hermes_gsp_set_state(false, HERMES_PHASE_OFFLINE);
	return admitted ? 0 : -EINVAL;
}

int hermes_firmware_measure(u32 byte_length, const u8 *sha256)
{
	int err;

	mutex_lock(&hermes_char_lock);
	err = __hermes_firmware_measure(byte_length, sha256);
	mutex_unlock(&hermes_char_lock);
	return err;
}
EXPORT_SYMBOL_GPL(hermes_firmware_measure);

bool hermes_firmware_is_admitted(void)
{
	return READ_ONCE(hermes_fw_admitted);
}
EXPORT_SYMBOL_GPL(hermes_firmware_is_admitted);

#if LINUX_VERSION_CODE >= KERNEL_VERSION(6, 2, 0)
static char *hermes_char_devnode(const struct device *dev, umode_t *mode)
#else
static char *hermes_char_devnode(struct device *dev, umode_t *mode)
#endif
{
	if (mode)
		*mode = 0666;
	return NULL;
}

/*
 * Probe open-stack companion modules via /sys/module/<name>.
 * Prefer sysfs over find_module: module_mutex is not exported to modules
 * on modern kernels. Soft-deps may be absent; never invents Online.
 */
static bool hermes_kernel_module_live(const char *name)
{
	char path[64];
	struct path p;
	int err;

	if (!name || !*name)
		return false;
	/* Kernel object names use underscores (nvidia_drm, not nvidia-drm). */
	scnprintf(path, sizeof(path), "/sys/module/%s", name);
	err = kern_path(path, LOOKUP_FOLLOW, &p);
	if (err)
		return false;
	path_put(&p);
	return true;
}

static unsigned hermes_live_module_mask(void)
{
	/*
	 * Kernel object names use '_' (nvidia_drm.ko → nvidia_drm).
	 * Classic drop-in filenames may use '-' — mask bits are filename-agnostic.
	 */
	return hermes_ctl_module_mask_compose(
		hermes_kernel_module_live("nvidia_modeset") ? 1 : 0,
		hermes_kernel_module_live("nvidia_uvm") ? 1 : 0,
		hermes_kernel_module_live("nvidia_drm") ? 1 : 0,
		hermes_kernel_module_live("nvidia_peermem") ? 1 : 0);
}

/* Format classic names for read() status line (comma-separated). */
static int hermes_format_modules(char *buf, size_t len, unsigned mask)
{
	int n = 0;
	int first = 1;

	if (!buf || len == 0)
		return 0;
	buf[0] = '\0';

#define APPEND(bit, name)                                                      \
	do {                                                                   \
		if ((mask) & (bit)) {                                          \
			n += scnprintf((buf) + n, (len) - (size_t)n, "%s%s",   \
				       first ? "" : ",", (name));              \
			first = 0;                                             \
		}                                                              \
	} while (0)

	APPEND(HERMES_MOD_NVIDIA, "nvidia");
	APPEND(HERMES_MOD_MODESET, "nvidia-modeset");
	APPEND(HERMES_MOD_UVM, "nvidia-uvm");
	APPEND(HERMES_MOD_DRM, "nvidia-drm");
	APPEND(HERMES_MOD_PEERMEM, "nvidia-peermem");
#undef APPEND
	return n;
}

static int hermes_char_open(struct inode *inode, struct file *file)
{
	unsigned int minor = iminor(inode);

	if (minor >= HERMES_CHAR_COUNT)
		return -ENODEV;
	file->private_data = (void *)(unsigned long)minor;
	/* Allow open even when Offline so userspace can query status. */
	return 0;
}

static int hermes_char_release(struct inode *inode, struct file *file)
{
	return 0;
}

static int hermes_pick_turing_display(struct hermes_pci_id *id)
{
	struct pci_dev *pdev = NULL;

	while ((pdev = pci_get_device(PCI_VENDOR_ID_NVIDIA, PCI_ANY_ID, pdev)) !=
	       NULL) {
		u8 class_code = (pdev->class >> 16) & 0xff;

		if (!hermes_is_turing_or_newer(pdev->device) || class_code != 0x03)
			continue;
		id->vendor = pdev->vendor;
		id->device = pdev->device;
		id->class_code = class_code;
		id->subclass = (pdev->class >> 8) & 0xff;
		pci_dev_put(pdev);
		return 0;
	}
	return -ENODEV;
}

static int hermes_run_sticky_bringup(struct hermes_bringup_result *out)
{
	struct hermes_pci_id id;
	struct hermes_bringup_result r;
	int err;

	err = hermes_pick_turing_display(&id);
	if (err)
		return err;
	r = hermes_run_bringup(&id, &hermes_sticky_ev);
	hermes_gsp_set_state(r.online, r.phase);
	if (out)
		*out = r;
	return 0;
}

static int hermes_sim_promote(void)
{
	struct hermes_bringup_result r;
	int err;

	if (!hermes_allow_sim_promote) {
		pr_info("hermes/nvidia: SIM_PROMOTE denied (allow_sim_promote=0)\n");
		return -EPERM;
	}
	hermes_sticky_ev.iommu_isolated = true;
	hermes_sticky_ev.dma_domain = 1;
	hermes_sticky_ev.wpr_locked = true;
	hermes_sticky_ev.mailbox_ok = true;
	hermes_sticky_ev.ready_ok = true;
	hermes_sticky_ev.firmware_measured = true;
	hermes_fw_admitted = true;
	err = hermes_run_sticky_bringup(&r);
	if (err)
		return err;
	pr_warn("hermes/nvidia: SIM_PROMOTE online=%d phase=%s (not silicon measure)\n",
		r.online, hermes_phase_name(r.phase));
	return r.online ? 0 : -EIO;
}

static int hermes_measure_fw(struct hermes_measure_fw *m)
{
	struct hermes_bringup_result r;
	int err;

	if (!m)
		return -EINVAL;
	err = __hermes_firmware_measure(m->byte_length, m->sha256);
	m->admitted = (err == 0) ? 1 : 0;
	if (err) {
		pr_info("hermes/nvidia: MEASURE_FW reject len=%u (not in pin list)\n",
			m->byte_length);
		m->phase = hermes_gsp_phase();
		m->online = hermes_gsp_is_online() ? 1 : 0;
		m->status = HERMES_BRINGUP_FIRMWARE;
	}
	if (err)
		return err;
	err = hermes_run_sticky_bringup(&r);
	if (err)
		return err;
	m->phase = r.phase;
	m->online = r.online ? 1 : 0;
	m->status = r.status;
	pr_info("hermes/nvidia: MEASURE_FW admit len=%u phase=%s online=%d\n",
		m->byte_length, hermes_phase_name(r.phase), r.online);
	return 0;
}

static int hermes_apply_evidence(struct hermes_apply_evidence *e)
{
	struct hermes_bringup_result r;
	int err;

	if (!e)
		return -EINVAL;
	if (e->force_fw_measured) {
		if (!hermes_allow_sim_promote)
			return -EPERM;
		hermes_sticky_ev.firmware_measured = true;
		hermes_fw_admitted = true;
	} else if (e->use_measured_fw) {
		if (!hermes_fw_admitted)
			return -EINVAL;
		hermes_sticky_ev.firmware_measured = true;
	}
	hermes_sticky_ev.iommu_isolated = e->iommu_isolated != 0;
	hermes_sticky_ev.dma_domain = e->dma_domain;
	hermes_sticky_ev.wpr_locked = e->wpr_locked != 0;
	hermes_sticky_ev.mailbox_ok = e->mailbox_ok != 0;
	hermes_sticky_ev.ready_ok = e->ready_ok != 0;
	err = hermes_run_sticky_bringup(&r);
	if (err)
		return err;
	e->phase = r.phase;
	e->online = r.online ? 1 : 0;
	e->status = r.status;
	pr_info("hermes/nvidia: APPLY_EVIDENCE phase=%s online=%d status=%d\n",
		hermes_phase_name(r.phase), r.online, r.status);
	return 0;
}

static long hermes_char_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
	struct hermes_ctl_status st;
	struct hermes_measure_fw mfw;
	struct hermes_apply_evidence aev;
	unsigned mask;
	int err = 0;

	(void)file;

	if (cmd == HERMES_CTL_IOCTL_SIM_PROMOTE) {
		mutex_lock(&hermes_char_lock);
		err = hermes_sim_promote();
		mutex_unlock(&hermes_char_lock);
		return err;
	}
	if (cmd == HERMES_CTL_IOCTL_DEMOTE) {
		mutex_lock(&hermes_char_lock);
		hermes_gsp_set_state(false, HERMES_PHASE_OFFLINE);
		hermes_fw_admitted = false;
		memset(&hermes_sticky_ev, 0, sizeof(hermes_sticky_ev));
		pr_info("hermes/nvidia: DEMOTE → Offline (evidence cleared)\n");
		mutex_unlock(&hermes_char_lock);
		return 0;
	}
	if (cmd == HERMES_CTL_IOCTL_MEASURE_FW) {
		if (copy_from_user(&mfw, (void __user *)arg, sizeof(mfw)))
			return -EFAULT;
		mutex_lock(&hermes_char_lock);
		err = hermes_measure_fw(&mfw);
		mutex_unlock(&hermes_char_lock);
		if (copy_to_user((void __user *)arg, &mfw, sizeof(mfw)))
			return -EFAULT;
		return err;
	}
	if (cmd == HERMES_CTL_IOCTL_APPLY_EVIDENCE) {
		if (copy_from_user(&aev, (void __user *)arg, sizeof(aev)))
			return -EFAULT;
		mutex_lock(&hermes_char_lock);
		err = hermes_apply_evidence(&aev);
		mutex_unlock(&hermes_char_lock);
		if (copy_to_user((void __user *)arg, &aev, sizeof(aev)))
			return -EFAULT;
		return err;
	}
	if (cmd != HERMES_CTL_IOCTL_STATUS)
		return -ENOTTY;

	mutex_lock(&hermes_char_lock);
	/* Primary + live companions; Online follows the published GSP phase. */
	mask = hermes_live_module_mask();
	hermes_ctl_status_fill(&st, hermes_gsp_is_online() ? 1 : 0,
			       (unsigned)hermes_gsp_phase(), mask);
	mutex_unlock(&hermes_char_lock);

	if (copy_to_user((void __user *)arg, &st, sizeof(st)))
		return -EFAULT;
	return 0;
}

static ssize_t hermes_char_read(struct file *file, char __user *buf, size_t len,
				loff_t *ppos)
{
	char mods[96];
	char line[192];
	int n;
	unsigned mask;

	if (*ppos != 0)
		return 0;

	mask = hermes_live_module_mask();
	hermes_format_modules(mods, sizeof(mods), mask);
	n = scnprintf(line, sizeof(line),
		      "hermes gsp_online=%d phase=%s modules=%s status_ver=%u mask=0x%x\n",
		      hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()),
		      mods, HERMES_CTL_STATUS_VERSION, mask);
	if (len < (size_t)n)
		return -EINVAL;
	if (copy_to_user(buf, line, n))
		return -EFAULT;
	*ppos = n;
	return n;
}

static const struct file_operations hermes_char_fops = {
	.owner = THIS_MODULE,
	.open = hermes_char_open,
	.release = hermes_char_release,
	.unlocked_ioctl = hermes_char_ioctl,
#ifdef CONFIG_COMPAT
	.compat_ioctl = hermes_char_ioctl,
#endif
	.read = hermes_char_read,
	.llseek = noop_llseek,
};

int hermes_chardev_init(void)
{
	int err;
	int i;

	err = alloc_chrdev_region(&hermes_char_devt, 0, HERMES_CHAR_COUNT, "nvidia");
	if (err)
		return err;

	cdev_init(&hermes_char_cdev, &hermes_char_fops);
	hermes_char_cdev.owner = THIS_MODULE;
	err = cdev_add(&hermes_char_cdev, hermes_char_devt, HERMES_CHAR_COUNT);
	if (err)
		goto err_region;

	hermes_char_class = class_create("nvidia");
	if (IS_ERR(hermes_char_class)) {
		err = PTR_ERR(hermes_char_class);
		goto err_cdev;
	}
	hermes_char_class->devnode = hermes_char_devnode;

	for (i = 0; i < HERMES_CHAR_COUNT; i++) {
		const char *name = (i == 0) ? HERMES_CHAR_NAME_CTL : HERMES_CHAR_NAME_0;
		struct device *d;

		d = device_create(hermes_char_class, NULL,
				  MKDEV(MAJOR(hermes_char_devt), i), NULL, "%s", name);
		if (IS_ERR(d)) {
			err = PTR_ERR(d);
			while (--i >= 0)
				device_destroy(hermes_char_class,
					      MKDEV(MAJOR(hermes_char_devt), i));
			goto err_class;
		}
	}

	pr_info("hermes/nvidia: char nodes /dev/%s /dev/%s (gsp_online=%d)\n",
		HERMES_CHAR_NAME_CTL, HERMES_CHAR_NAME_0, hermes_gsp_is_online());
	return 0;

err_class:
	class_destroy(hermes_char_class);
err_cdev:
	cdev_del(&hermes_char_cdev);
err_region:
	unregister_chrdev_region(hermes_char_devt, HERMES_CHAR_COUNT);
	return err;
}

void hermes_chardev_exit(void)
{
	int i;

	for (i = 0; i < HERMES_CHAR_COUNT; i++)
		device_destroy(hermes_char_class, MKDEV(MAJOR(hermes_char_devt), i));
	class_destroy(hermes_char_class);
	cdev_del(&hermes_char_cdev);
	unregister_chrdev_region(hermes_char_devt, HERMES_CHAR_COUNT);
	pr_info("hermes/nvidia: char nodes removed\n");
}
