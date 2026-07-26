// SPDX-License-Identifier: MIT
#include <linux/module.h>

static int __init hermes_uvm_init(void)
{
	pr_info("hermes/nvidia-uvm: companion surface loaded (fail-closed until GSP online)\n");
	return 0;
}

static void __exit hermes_uvm_exit(void)
{
	pr_info("hermes/nvidia-uvm: unloaded\n");
}

module_init(hermes_uvm_init);
module_exit(hermes_uvm_exit);
MODULE_DESCRIPTION("Hermes UVM companion (module name: nvidia-uvm)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
