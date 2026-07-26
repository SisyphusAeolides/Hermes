// SPDX-License-Identifier: MIT
#include <linux/module.h>
#include "include/hermes_kmod.h"

extern bool hermes_gsp_is_online(void);
extern enum hermes_phase hermes_gsp_phase(void);

static int __init hermes_modeset_init(void)
{
	bool online = hermes_gsp_is_online();

	pr_info("hermes/nvidia-modeset: loaded (gsp_online=%d phase=%s); modeset gated\n",
		online, hermes_phase_name(hermes_gsp_phase()));
	if (!online)
		pr_info("hermes/nvidia-modeset: Offline — no modeset authority published\n");
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
