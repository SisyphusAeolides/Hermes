/* Host unit test for hermes_ctl_status packing + companion mask compose. */
#include <assert.h>
#include <stdio.h>
#include <string.h>

#define HERMES_HOST_TEST 1
#include "include/hermes_ctl_uapi.h"

int main(void)
{
	struct hermes_ctl_status st;
	unsigned m;

	memset(&st, 0, sizeof(st));
	hermes_ctl_status_fill(&st, 0, 0 /* OFFLINE */, HERMES_MOD_NVIDIA);
	assert(st.gsp_online == 0);
	assert(st.phase == 0);
	assert(st.version == HERMES_CTL_STATUS_VERSION);
	assert(st.module_mask & HERMES_MOD_NVIDIA);

	hermes_ctl_status_fill(&st, 1, 5 /* ONLINE */,
			       HERMES_MOD_NVIDIA | HERMES_MOD_DRM);
	assert(st.gsp_online == 1);
	assert(st.phase == 5);
	assert(st.module_mask & HERMES_MOD_DRM);

	/* Companion OR: primary alone */
	m = hermes_ctl_module_mask_compose(0, 0, 0, 0);
	assert(m == HERMES_MOD_NVIDIA);

	/* All companions live */
	m = hermes_ctl_module_mask_compose(1, 1, 1, 1);
	assert(m == HERMES_MOD_ALL_OPEN_STACK);
	assert(m & HERMES_MOD_MODESET);
	assert(m & HERMES_MOD_UVM);
	assert(m & HERMES_MOD_DRM);
	assert(m & HERMES_MOD_PEERMEM);

	/* Partial: modeset + drm only */
	m = hermes_ctl_module_mask_compose(1, 0, 1, 0);
	assert(m == (HERMES_MOD_NVIDIA | HERMES_MOD_MODESET | HERMES_MOD_DRM));
	assert(!(m & HERMES_MOD_UVM));
	assert(!(m & HERMES_MOD_PEERMEM));

	printf("test_ctl_status_host: PASS\n");
	return 0;
}
