/* Host unit test: companion modules must gate on GSP Online policy. */
#include <assert.h>
#include <stdio.h>
#include <stdbool.h>

/* Pure policy mirror of companion ioctl gate (no kernel). */
static int companion_ioctl_errno(bool gsp_online)
{
	return gsp_online ? -25 /* ENOTTY placeholder when Online */ : -19 /* ENODEV */;
}

int main(void)
{
	assert(companion_ioctl_errno(false) == -19);
	assert(companion_ioctl_errno(true) == -25);
	printf("test_companion_gate_host: PASS\n");
	return 0;
}
