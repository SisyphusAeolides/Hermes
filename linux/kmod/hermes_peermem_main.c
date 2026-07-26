// SPDX-License-Identifier: MIT
/*
 * Hermes peermem companion — module name nvidia-peermem.
 * Registration API gated on hermes_gsp_is_online(); status misc node for operators.
 */

#include <linux/module.h>
#include <linux/export.h>
#include <linux/fs.h>
#include <linux/miscdevice.h>
#include <linux/uaccess.h>
#include <linux/ioctl.h>

#include "include/hermes_kmod.h"
#include "include/hermes_ctl_uapi.h"
#include "include/hermes_companion_uapi.h"

extern bool hermes_gsp_is_online(void);
extern enum hermes_phase hermes_gsp_phase(void);

/**
 * hermes_peermem_register_ok - whether peer memory registration is authorized.
 * Returns true only when primary GSP module reports Online.
 */
bool hermes_peermem_register_ok(void)
{
	return hermes_gsp_is_online();
}
EXPORT_SYMBOL_GPL(hermes_peermem_register_ok);

static long hermes_peermem_ioctl(struct file *file, unsigned int cmd,
				 unsigned long arg)
{
	struct hermes_ctl_status st;

	(void)file;
	if (cmd != HERMES_COMPANION_IOCTL_STATUS)
		return -ENOTTY;
	hermes_ctl_status_fill(&st, hermes_gsp_is_online() ? 1 : 0,
			       (unsigned)hermes_gsp_phase(), HERMES_MOD_PEERMEM);
	if (copy_to_user((void __user *)arg, &st, sizeof(st)))
		return -EFAULT;
	return 0;
}

static ssize_t hermes_peermem_read(struct file *file, char __user *buf, size_t len,
				   loff_t *ppos)
{
	char line[112];
	int n;

	(void)file;
	if (*ppos != 0)
		return 0;
	n = scnprintf(line, sizeof(line),
		      "hermes/nvidia-peermem gsp_online=%d phase=%s register_ok=%d\n",
		      hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()),
		      hermes_peermem_register_ok() ? 1 : 0);
	if (len < (size_t)n)
		return -EINVAL;
	if (copy_to_user(buf, line, n))
		return -EFAULT;
	*ppos = n;
	return n;
}

static const struct file_operations hermes_peermem_fops = {
	.owner = THIS_MODULE,
	.unlocked_ioctl = hermes_peermem_ioctl,
#ifdef CONFIG_COMPAT
	.compat_ioctl = hermes_peermem_ioctl,
#endif
	.read = hermes_peermem_read,
	.llseek = noop_llseek,
};

static struct miscdevice hermes_peermem_misc = {
	.minor = MISC_DYNAMIC_MINOR,
	.name = "nvidia-peermem",
	.fops = &hermes_peermem_fops,
	.mode = 0666,
};

static int __init hermes_peermem_init(void)
{
	int err;

	err = misc_register(&hermes_peermem_misc);
	if (err) {
		pr_err("hermes/nvidia-peermem: misc_register failed: %d\n", err);
		return err;
	}
	pr_info("hermes/nvidia-peermem: /dev/nvidia-peermem ready (gsp_online=%d)\n",
		hermes_gsp_is_online());
	return 0;
}

static void __exit hermes_peermem_exit(void)
{
	misc_deregister(&hermes_peermem_misc);
	pr_info("hermes/nvidia-peermem: unloaded\n");
}

module_init(hermes_peermem_init);
module_exit(hermes_peermem_exit);
MODULE_DESCRIPTION("Hermes peermem companion (module name: nvidia-peermem)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
