// SPDX-License-Identifier: MIT
#include <linux/module.h>

static int __init hermes_drm_init(void)
{
	pr_info("hermes/nvidia-drm: companion surface loaded (fail-closed until GSP online)\n");
	return 0;
}

static void __exit hermes_drm_exit(void)
{
	pr_info("hermes/nvidia-drm: unloaded\n");
}

module_init(hermes_drm_init);
module_exit(hermes_drm_exit);
MODULE_DESCRIPTION("Hermes DRM companion (module name: nvidia-drm)");
MODULE_AUTHOR("SisyphusAeolides");
MODULE_LICENSE("Dual MIT/GPL");
MODULE_SOFTDEP("pre: nvidia");
