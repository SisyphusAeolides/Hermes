// SPDX-License-Identifier: MIT
#include <linux/module.h>

static int __init hermes_peermem_init(void)
{
	pr_info("hermes/nvidia-peermem: companion surface loaded (fail-closed until GSP online)\n");
	return 0;
}

static void __exit hermes_peermem_exit(void)
{
	pr_info("hermes/nvidia-peermem: unloaded\n");
}

module_init(hermes_peermem_init);
module_exit(hermes_peermem_exit);
MODULE_DESCRIPTION("Hermes peermem companion (module name: nvidia-peermem)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
