// SPDX-License-Identifier: MIT
/*
 * Hermes modeset companion — module name nvidia-modeset.
 * /dev/nvidia-modeset char surface; ops require GSP Online.
 */

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/uaccess.h>

#include "include/hermes_kmod.h"

extern bool hermes_gsp_is_online(void);
extern enum hermes_phase hermes_gsp_phase(void);

static long hermes_modeset_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
	(void)file;
	(void)cmd;
	(void)arg;
	if (!hermes_gsp_is_online())
		return -ENODEV;
	return -ENOTTY;
}

static ssize_t hermes_modeset_read(struct file *file, char __user *buf, size_t len,
				   loff_t *ppos)
{
	char line[96];
	int n;

	(void)file;
	if (*ppos != 0)
		return 0;
	n = scnprintf(line, sizeof(line),
		      "hermes/nvidia-modeset gsp_online=%d phase=%s\n",
		      hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()));
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
	pr_info("hermes/nvidia-modeset: /dev/nvidia-modeset ready (gsp_online=%d)\n",
		hermes_gsp_is_online());
	return 0;
}

static void __exit hermes_modeset_exit(void)
{
	misc_deregister(&hermes_modeset_misc);
	pr_info("hermes/nvidia-modeset: unloaded\n");
}

module_init(hermes_modeset_init);
module_exit(hermes_modeset_exit);
MODULE_DESCRIPTION("Hermes modeset companion (module name: nvidia-modeset)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
