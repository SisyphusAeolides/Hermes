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

	/* Firmware pin match (610.43.02 tu10x digest). */
	{
		struct hermes_fw_pin pins[1];
		__u8 sha[32] = {
			0xc8, 0xfc, 0x1a, 0x92, 0xc9, 0x0b, 0x03, 0x4b, 0xbb, 0xe4,
			0xd5, 0x6c, 0xa9, 0x4b, 0x0d, 0xc9, 0x5a, 0xfb, 0x52, 0xd3,
			0x40, 0x9a, 0x78, 0x80, 0x18, 0x6a, 0xe0, 0x3c, 0x7d, 0xde,
			0x17, 0xf3
		};
		__u8 bad[32] = { 0 };

		pins[0].byte_length = 29352832;
		memcpy(pins[0].sha256, sha, 32);
		assert(hermes_fw_pin_match(pins, 1, 29352832, sha) == 1);
		assert(hermes_fw_pin_match(pins, 1, 29352832, bad) == 0);
		assert(hermes_fw_pin_match(pins, 1, 1, sha) == 0);
	}

	printf("test_ctl_status_host: PASS\n");
	return 0;
}
