/* Host unit test for hermes_ctl_status packing. */
#include <assert.h>
#include <stdio.h>
#include <string.h>

#define HERMES_HOST_TEST 1
#include "include/hermes_ctl_uapi.h"

int main(void)
{
	struct hermes_ctl_status st;

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

	printf("test_ctl_status_host: PASS\n");
	return 0;
}
