// SPDX-License-Identifier: MIT
/*
 * Hermes UVM companion — nvidia-uvm + nvidia-uvm-tools.
 * STATUS always; INITIALIZE/REGISTER/PAGEABLE require GSP Online.
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

static DEFINE_MUTEX(hermes_uvm_lock);
static bool hermes_uvm_initialized;
static bool hermes_uvm_gpu_registered;
static __u32 hermes_uvm_pageable = 1; /* software capability when Online */

static long hermes_uvm_status_ioctl(struct file *file, unsigned int cmd,
				    unsigned long arg)
{
	struct hermes_ctl_status st;

	(void)file;
	if (cmd != HERMES_UVM_IOCTL_STATUS)
		return -ENOTTY;
	hermes_ctl_status_fill(&st, hermes_gsp_is_online() ? 1 : 0,
			       (unsigned)hermes_gsp_phase(), HERMES_MOD_UVM);
	if (copy_to_user((void __user *)arg, &st, sizeof(st)))
		return -EFAULT;
	return 0;
}

static long hermes_uvm_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
	struct hermes_uvm_init init;
	struct hermes_uvm_register_gpu reg;
	__u32 pageable;
	__u32 gpu_id;
	long err = 0;

	if (cmd == HERMES_UVM_IOCTL_STATUS)
		return hermes_uvm_status_ioctl(file, cmd, arg);

	if (!hermes_gsp_is_online())
		return -ENODEV;

	mutex_lock(&hermes_uvm_lock);
	switch (cmd) {
	case HERMES_UVM_IOCTL_INITIALIZE:
		if (copy_from_user(&init, (void __user *)arg, sizeof(init))) {
			err = -EFAULT;
			break;
		}
		hermes_uvm_initialized = true;
		pr_info("hermes/nvidia-uvm: INITIALIZE flags=0x%x\n", init.flags);
		break;
	case HERMES_UVM_IOCTL_PAGEABLE_MEM_ACCESS:
		if (!hermes_uvm_initialized) {
			err = -EINVAL;
			break;
		}
		pageable = hermes_uvm_pageable;
		if (copy_to_user((void __user *)arg, &pageable, sizeof(pageable)))
			err = -EFAULT;
		break;
	case HERMES_UVM_IOCTL_REGISTER_GPU:
		if (!hermes_uvm_initialized) {
			err = -EINVAL;
			break;
		}
		if (copy_from_user(&reg, (void __user *)arg, sizeof(reg))) {
			err = -EFAULT;
			break;
		}
		hermes_uvm_gpu_registered = true;
		reg.registered = 1;
		if (copy_to_user((void __user *)arg, &reg, sizeof(reg)))
			err = -EFAULT;
		else
			pr_info("hermes/nvidia-uvm: REGISTER_GPU ok\n");
		break;
	case HERMES_UVM_IOCTL_UNREGISTER_GPU:
		if (copy_from_user(&gpu_id, (void __user *)arg, sizeof(gpu_id))) {
			err = -EFAULT;
			break;
		}
		hermes_uvm_gpu_registered = false;
		(void)gpu_id;
		break;
	default:
		err = -ENOTTY;
		break;
	}
	mutex_unlock(&hermes_uvm_lock);
	return err;
}

static ssize_t hermes_uvm_read(struct file *file, char __user *buf, size_t len,
			       loff_t *ppos)
{
	char line[128];
	int n;

	(void)file;
	if (*ppos != 0)
		return 0;
	n = scnprintf(line, sizeof(line),
		      "hermes/nvidia-uvm gsp_online=%d phase=%s init=%d gpu_reg=%d\n",
		      hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()),
		      hermes_uvm_initialized ? 1 : 0,
		      hermes_uvm_gpu_registered ? 1 : 0);
	if (len < (size_t)n)
		return -EINVAL;
	if (copy_to_user(buf, line, n))
		return -EFAULT;
	*ppos = n;
	return n;
}

static const struct file_operations hermes_uvm_fops = {
	.owner = THIS_MODULE,
	.unlocked_ioctl = hermes_uvm_ioctl,
#ifdef CONFIG_COMPAT
	.compat_ioctl = hermes_uvm_ioctl,
#endif
	.read = hermes_uvm_read,
	.llseek = noop_llseek,
};

static const struct file_operations hermes_uvm_tools_fops = {
	.owner = THIS_MODULE,
	.unlocked_ioctl = hermes_uvm_status_ioctl,
#ifdef CONFIG_COMPAT
	.compat_ioctl = hermes_uvm_status_ioctl,
#endif
	.read = hermes_uvm_read,
	.llseek = noop_llseek,
};

static struct miscdevice hermes_uvm_misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "nvidia-uvm",
	.fops = &hermes_uvm_fops,
	.mode = 0666,
};

static struct miscdevice hermes_uvm_tools_misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "nvidia-uvm-tools",
	.fops = &hermes_uvm_tools_fops,
	.mode = 0666,
};

static int __init hermes_uvm_init(void)
{
	int err;

	err = misc_register(&hermes_uvm_misc);
	if (err) {
		pr_err("hermes/nvidia-uvm: misc_register failed: %d\n", err);
		return err;
	}
	err = misc_register(&hermes_uvm_tools_misc);
	if (err) {
		misc_deregister(&hermes_uvm_misc);
		return err;
	}
	pr_info("hermes/nvidia-uvm: ready (gsp_online=%d)\n", hermes_gsp_is_online());
	return 0;
}

static void __exit hermes_uvm_exit(void)
{
	misc_deregister(&hermes_uvm_tools_misc);
	misc_deregister(&hermes_uvm_misc);
}

module_init(hermes_uvm_init);
module_exit(hermes_uvm_exit);
MODULE_DESCRIPTION("Hermes UVM companion (module name: nvidia-uvm)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
