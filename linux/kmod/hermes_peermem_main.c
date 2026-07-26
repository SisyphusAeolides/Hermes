// SPDX-License-Identifier: MIT
/*
 * Hermes peermem companion — module name nvidia-peermem.
 * Registration API gated on hermes_gsp_is_online().
 */

#include <linux/module.h>
#include <linux/export.h>

#include "include/hermes_kmod.h"

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

static int __init hermes_peermem_init(void)
{
	pr_info("hermes/nvidia-peermem: loaded (gsp_online=%d phase=%s); register gated\n",
		hermes_gsp_is_online(), hermes_phase_name(hermes_gsp_phase()));
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
