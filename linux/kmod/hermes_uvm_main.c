// SPDX-License-Identifier: MIT
/*
 * Hermes UVM companion — module name nvidia-uvm.
 * /dev/nvidia-uvm + /dev/nvidia-uvm-tools; ops gated on GSP Online.
 */

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/uaccess.h>
#include <linux/ioctl.h>

#include "include/hermes_kmod.h"
#include "include/hermes_ctl_uapi.h"
#include "include/hermes_companion_uapi.h"

extern bool hermes_gsp_is_online(void);
extern enum hermes_phase hermes_gsp_phase(void);

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
	if (cmd == HERMES_UVM_IOCTL_STATUS)
		return hermes_uvm_status_ioctl(file, cmd, arg);
	if (!hermes_gsp_is_online())
		return -ENODEV;
	/* Full UVM ioctl set is future work; Online authorizes the surface. */
	return -ENOTTY;
}

static ssize_t hermes_uvm_read(struct file *file, char __user *buf, size_t len,
			       loff_t *ppos)
{
	char line[96];
	int n;

	(void)file;
	if (*ppos != 0)
		return 0;
	n = scnprintf(line, sizeof(line),
		      "hermes/nvidia-uvm gsp_online=%d phase=%s\n",
		      hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()));
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
		pr_err("hermes/nvidia-uvm: tools register failed: %d\n", err);
		misc_deregister(&hermes_uvm_misc);
		return err;
	}
	pr_info("hermes/nvidia-uvm: /dev/nvidia-uvm + tools ready (gsp_online=%d)\n",
		hermes_gsp_is_online());
	return 0;
}

static void __exit hermes_uvm_exit(void)
{
	misc_deregister(&hermes_uvm_tools_misc);
	misc_deregister(&hermes_uvm_misc);
	pr_info("hermes/nvidia-uvm: unloaded\n");
}

module_init(hermes_uvm_init);
module_exit(hermes_uvm_exit);
MODULE_DESCRIPTION("Hermes UVM companion (module name: nvidia-uvm)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
