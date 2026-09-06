/* Host unit test for hermes_run_bringup — compiles without kernel headers. */
#include <stdio.h>
#include <stdbool.h>

#define HERMES_HOST_TEST 1
#include "../include/hermes_kmod.h"

/* Link against hermes_bringup.c compiled with -DHERMES_HOST_TEST */

static int failures;

static void expect_true(const char *name, bool cond)
{
	if (!cond) {
		fprintf(stderr, "FAIL %s\n", name);
		failures++;
	} else {
		printf("ok %s\n", name);
	}
}

int main(void)
{
	struct hermes_pci_id t1000 = {
		.vendor = 0x10de, .device = 0x1fb9, .class_code = 0x03, .subclass = 0
	};
	struct hermes_pci_id volta = {
		.vendor = 0x10de, .device = 0x1db6, .class_code = 0x03, .subclass = 0
	};
	struct hermes_hw_evidence empty = { 0 };
	struct hermes_hw_evidence full = {
		.iommu_isolated = true,
		.dma_domain = 7,
		.wpr_locked = true,
		.mailbox_ok = true,
		.ready_ok = true,
		.firmware_measured = true,
	};
	struct hermes_hw_evidence no_wpr = full;
	struct hermes_bringup_result r;

	no_wpr.wpr_locked = false;

	expect_true("t1000 turing+", hermes_is_turing_or_newer(0x1fb9));
	expect_true("volta not turing+", !hermes_is_turing_or_newer(0x1db6));

	r = hermes_run_bringup(&volta, &full);
	expect_true("volta reject", !r.online && r.status == HERMES_BRINGUP_PRE_TURING);

	r = hermes_run_bringup(&t1000, &empty);
	expect_true("empty evidence offline", !r.online && r.phase != HERMES_PHASE_ONLINE);

	r = hermes_run_bringup(&t1000, &no_wpr);
	expect_true("no wpr offline",
		    !r.online && r.status == HERMES_BRINGUP_INCOMPLETE_EVIDENCE);

	r = hermes_run_bringup(&t1000, &full);
	expect_true("full evidence online",
		    r.online && r.phase == HERMES_PHASE_ONLINE &&
			    r.status == HERMES_BRINGUP_OK);

	return failures ? 1 : 0;
}
