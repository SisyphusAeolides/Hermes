// SPDX-License-Identifier: MIT
#include <linux/module.h>
#include "include/hermes_kmod.h"

static int __init hermes_modeset_init(void)
{
	pr_info("hermes/nvidia-modeset: companion surface loaded (depends on nvidia GSP online)\n");
	return 0;
}

static void __exit hermes_modeset_exit(void)
{
	pr_info("hermes/nvidia-modeset: unloaded\n");
}

module_init(hermes_modeset_init);
module_exit(hermes_modeset_exit);
MODULE_DESCRIPTION("Hermes modeset companion (module name: nvidia-modeset)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
