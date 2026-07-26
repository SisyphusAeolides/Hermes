// SPDX-License-Identifier: MIT
/*
 * Hermes modeset companion — nvidia-modeset.
 * STATUS always; ALLOC/FLIP/FREE require GSP Online.
 */

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/uaccess.h>
#include <linux/mutex.h>
#include <linux/ioctl.h>

#include "include/hermes_kmod.h"
#include "include/hermes_ctl_uapi.h"
#include "include/hermes_companion_uapi.h"

extern bool hermes_gsp_is_online(void);
extern enum hermes_phase hermes_gsp_phase(void);

static DEFINE_MUTEX(hermes_modeset_lock);
static __u32 hermes_ms_next_handle = 1;
static __u32 hermes_ms_sequence;
static __u32 hermes_ms_live_handles;

static long hermes_modeset_ioctl(struct file *file, unsigned int cmd,
				 unsigned long arg)
{
	struct hermes_ctl_status st;
	struct hermes_modeset_alloc alloc;
	struct hermes_modeset_flip flip;
	__u32 handle;
	long err = 0;

	(void)file;
	if (cmd == HERMES_COMPANION_IOCTL_STATUS) {
		hermes_ctl_status_fill(&st, hermes_gsp_is_online() ? 1 : 0,
				       (unsigned)hermes_gsp_phase(),
				       HERMES_MOD_MODESET);
		if (copy_to_user((void __user *)arg, &st, sizeof(st)))
			return -EFAULT;
		return 0;
	}

	if (!hermes_gsp_is_online())
		return -ENODEV;

	mutex_lock(&hermes_modeset_lock);
	switch (cmd) {
	case HERMES_MODESET_IOCTL_ALLOC:
		if (copy_from_user(&alloc, (void __user *)arg, sizeof(alloc))) {
			err = -EFAULT;
			break;
		}
		if (alloc.width == 0 || alloc.height == 0) {
			err = -EINVAL;
			break;
		}
		alloc.handle = hermes_ms_next_handle++;
		if (hermes_ms_next_handle == 0)
			hermes_ms_next_handle = 1;
		hermes_ms_live_handles++;
		if (copy_to_user((void __user *)arg, &alloc, sizeof(alloc)))
			err = -EFAULT;
		break;
	case HERMES_MODESET_IOCTL_FLIP:
		if (copy_from_user(&flip, (void __user *)arg, sizeof(flip))) {
			err = -EFAULT;
			break;
		}
		if (flip.handle == 0 || flip.crtc_id == 0) {
			err = -EINVAL;
			break;
		}
		hermes_ms_sequence++;
		flip.sequence = hermes_ms_sequence;
		if (copy_to_user((void __user *)arg, &flip, sizeof(flip)))
			err = -EFAULT;
		break;
	case HERMES_MODESET_IOCTL_FREE:
		if (copy_from_user(&handle, (void __user *)arg, sizeof(handle))) {
			err = -EFAULT;
			break;
		}
		if (handle == 0) {
			err = -EINVAL;
			break;
		}
		if (hermes_ms_live_handles)
			hermes_ms_live_handles--;
		break;
	default:
		err = -ENOTTY;
		break;
	}
	mutex_unlock(&hermes_modeset_lock);
	return err;
}

static ssize_t hermes_modeset_read(struct file *file, char __user *buf, size_t len,
				   loff_t *ppos)
{
	char line[128];
	int n;

	(void)file;
	if (*ppos != 0)
		return 0;
	n = scnprintf(line, sizeof(line),
		      "hermes/nvidia-modeset gsp_online=%d phase=%s handles=%u seq=%u\n",
		      hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()),
		      hermes_ms_live_handles, hermes_ms_sequence);
	if (len < (size_t)n)
		return -EINVAL;
	if (copy_to_user(buf, line, n))
		return -EFAULT;
	*ppos = n;
	return n;
}

static const struct file_operations hermes_modeset_fops = {
	.owner = THIS_MODULE,
	.unlocked_ioctl = hermes_modeset_ioctl,
#ifdef CONFIG_COMPAT
	.compat_ioctl = hermes_modeset_ioctl,
#endif
	.read = hermes_modeset_read,
	.llseek = noop_llseek,
};

static struct miscdevice hermes_modeset_misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "nvidia-modeset",
	.fops = &hermes_modeset_fops,
	.mode = 0666,
};

static int __init hermes_modeset_init(void)
{
	int err;

	err = misc_register(&hermes_modeset_misc);
	if (err) {
		pr_err("hermes/nvidia-modeset: misc_register failed: %d\n", err);
		return err;
	}
	pr_info("hermes/nvidia-modeset: ready (gsp_online=%d)\n",
		hermes_gsp_is_online());
	return 0;
}

static void __exit hermes_modeset_exit(void)
{
	misc_deregister(&hermes_modeset_misc);
}

module_init(hermes_modeset_init);
module_exit(hermes_modeset_exit);
MODULE_DESCRIPTION("Hermes modeset companion (module name: nvidia-modeset)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
