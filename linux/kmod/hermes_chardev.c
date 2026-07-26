// SPDX-License-Identifier: MIT
/*
 * Classic NVIDIA character device names: /dev/nvidiactl, /dev/nvidia0.
 * Open/ioctl fail-closed while GSP is Offline. Does not invent device Online.
 */

#include <linux/module.h>
#include <linux/fs.h>
#include <linux/cdev.h>
#include <linux/device.h>
#include <linux/uaccess.h>
#include <linux/mutex.h>

#include "include/hermes_kmod.h"
#include "include/hermes_ctl_uapi.h"

#define HERMES_CHAR_NAME_CTL "nvidiactl"
#define HERMES_CHAR_NAME_0 "nvidia0"
#define HERMES_CHAR_COUNT 2

/* IOCtl base for Hermes RM-shaped control (not proprietary numbers). */
#define HERMES_CTL_IOCTL_BASE 0x48
#define HERMES_CTL_IOCTL_STATUS _IOR(HERMES_CTL_IOCTL_BASE, 0x10, struct hermes_ctl_status)

extern bool hermes_gsp_is_online(void);
extern enum hermes_phase hermes_gsp_phase(void);

static dev_t hermes_char_devt;
static struct class *hermes_char_class;
static struct cdev hermes_char_cdev;
static DEFINE_MUTEX(hermes_char_lock);

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

static long hermes_char_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
	struct hermes_ctl_status st;

	if (cmd != HERMES_CTL_IOCTL_STATUS)
		return -ENOTTY;

	mutex_lock(&hermes_char_lock);
	/* Primary nvidia.ko is loaded if this code runs; companions optional later. */
	hermes_ctl_status_fill(&st, hermes_gsp_is_online() ? 1 : 0,
			       (unsigned)hermes_gsp_phase(), HERMES_MOD_NVIDIA);
	mutex_unlock(&hermes_char_lock);

	if (copy_to_user((void __user *)arg, &st, sizeof(st)))
		return -EFAULT;
	return 0;
}

static ssize_t hermes_char_read(struct file *file, char __user *buf, size_t len,
				loff_t *ppos)
{
	char line[64];
	int n;

	if (*ppos != 0)
		return 0;
	n = scnprintf(line, sizeof(line),
		      "hermes gsp_online=%d phase=%s modules=nvidia status_ver=%u\n",
		      hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()),
		      HERMES_CTL_STATUS_VERSION);
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
