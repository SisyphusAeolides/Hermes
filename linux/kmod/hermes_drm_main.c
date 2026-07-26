// SPDX-License-Identifier: MIT
/*
 * Hermes DRM companion exported as module name "nvidia-drm".
 * Character device + ioctls gate on hermes_gsp_is_online() from nvidia.ko.
 * Full DRM subsystem registration is future work; this is the fail-closed
 * ioctl surface matching hermes-drm userspace logic.
 */

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/uaccess.h>
#include <linux/mutex.h>

#include "include/hermes_kmod.h"
#include "include/hermes_drm_uapi.h"

/* Exported by nvidia.ko (hermes_core / nvidia_main). */
extern bool hermes_gsp_is_online(void);

static DEFINE_MUTEX(hermes_drm_lock);
static struct hermes_drm_logic hermes_drm_state;

static long hermes_drm_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
	int err = 0;

	mutex_lock(&hermes_drm_lock);
	/* Resync Online from primary GSP module on every ioctl. */
	hermes_drm_state.gsp_online = hermes_gsp_is_online() ? 1 : 0;

	switch (cmd) {
	case HERMES_DRM_IOCTL_STATUS: {
		struct hermes_drm_status st;

		err = hermes_drm_logic_status(&hermes_drm_state, &st);
		if (err)
			break;
		if (copy_to_user((void __user *)arg, &st, sizeof(st)))
			err = -EFAULT;
		break;
	}
	case HERMES_DRM_IOCTL_DUMB_CREATE: {
		struct hermes_drm_dumb_create req;

		if (copy_from_user(&req, (void __user *)arg, sizeof(req))) {
			err = -EFAULT;
			break;
		}
		err = hermes_drm_logic_dumb_create(&hermes_drm_state, &req);
		if (err == HERMES_DRM_E_GSP_OFFLINE)
			err = -ENODEV;
		else if (err == HERMES_DRM_E_INVAL)
			err = -EINVAL;
		else if (err)
			err = -EIO;
		else if (copy_to_user((void __user *)arg, &req, sizeof(req)))
			err = -EFAULT;
		break;
	}
	case HERMES_DRM_IOCTL_ATOMIC: {
		struct hermes_drm_atomic_req req;

		if (copy_from_user(&req, (void __user *)arg, sizeof(req))) {
			err = -EFAULT;
			break;
		}
		err = hermes_drm_logic_atomic(&hermes_drm_state, &req);
		if (err == HERMES_DRM_E_GSP_OFFLINE)
			err = -ENODEV;
		else if (err == HERMES_DRM_E_INVAL)
			err = -EINVAL;
		else if (err)
			err = -EIO;
		else if (copy_to_user((void __user *)arg, &req, sizeof(req)))
			err = -EFAULT;
		break;
	}
	case HERMES_DRM_IOCTL_DISABLE_CRTC: {
		__u32 crtc;

		if (copy_from_user(&crtc, (void __user *)arg, sizeof(crtc))) {
			err = -EFAULT;
			break;
		}
		err = hermes_drm_logic_disable(&hermes_drm_state, crtc);
		if (err == HERMES_DRM_E_GSP_OFFLINE)
			err = -ENODEV;
		else if (err)
			err = -EINVAL;
		break;
	}
	default:
		err = -ENOTTY;
		break;
	}

	mutex_unlock(&hermes_drm_lock);
	return err;
}

static const struct file_operations hermes_drm_fops = {
	.owner = THIS_MODULE,
	.unlocked_ioctl = hermes_drm_ioctl,
#ifdef CONFIG_COMPAT
	.compat_ioctl = hermes_drm_ioctl,
#endif
};

static struct miscdevice hermes_drm_misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "nvidia-drm",
	.fops = &hermes_drm_fops,
	.mode = 0666,
};

static int __init hermes_drm_init(void)
{
	int err;

	hermes_drm_logic_init(&hermes_drm_state, hermes_gsp_is_online());
	err = misc_register(&hermes_drm_misc);
	if (err) {
		pr_err("hermes/nvidia-drm: misc_register failed: %d\n", err);
		return err;
	}
	pr_info("hermes/nvidia-drm: /dev/nvidia-drm ready (gsp_online=%d, fail-closed)\n",
		hermes_gsp_is_online());
	return 0;
}

static void __exit hermes_drm_exit(void)
{
	misc_deregister(&hermes_drm_misc);
	pr_info("hermes/nvidia-drm: unloaded\n");
}

module_init(hermes_drm_init);
module_exit(hermes_drm_exit);
MODULE_DESCRIPTION("Hermes DRM companion (module name: nvidia-drm)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
