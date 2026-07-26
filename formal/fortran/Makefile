# Compile Hermes Fortran formal models (gfortran).
# Force gfortran: some environments export FC=f77.
FC      := gfortran
FFLAGS  ?= -std=f2018 -Wall -Wextra -Wno-unused-function -O0 -g
MODDIR  := .
BUILD   := build

MODULES := \
	hermes_kinds.f90 \
	hermes_resources.f90 \
	hermes_rings.f90 \
	hermes_fail_closed.f90 \
	hermes_wpr.f90 \
	hermes_bootstrap.f90 \
	hermes_firmware.f90 \
	hermes_mailbox.f90 \
	hermes_host_gate.f90 \
	hermes_dropin.f90 \
	hermes_drm_kms.f90 \
	hermes_cccl.f90 \
	hermes_nvkm_gsp.f90

OBJS := $(addprefix $(BUILD)/,$(MODULES:.f90=.o))

.PHONY: all check clean

all: $(BUILD)/hermes_check

$(BUILD):
	mkdir -p $(BUILD)

$(BUILD)/%.o: %.f90 | $(BUILD)
	$(FC) $(FFLAGS) -J$(BUILD) -I$(BUILD) -c $< -o $@

# Module dependency order (manual DAG).
$(BUILD)/hermes_resources.o: $(BUILD)/hermes_kinds.o
$(BUILD)/hermes_rings.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_fail_closed.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_wpr.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_bootstrap.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_firmware.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_mailbox.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_host_gate.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_dropin.o: $(BUILD)/hermes_kinds.o
$(BUILD)/hermes_drm_kms.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_cccl.o: $(BUILD)/hermes_resources.o
$(BUILD)/hermes_nvkm_gsp.o: $(BUILD)/hermes_resources.o

$(BUILD)/hermes_check.o: hermes_check.f90 $(OBJS) | $(BUILD)
	$(FC) $(FFLAGS) -J$(BUILD) -I$(BUILD) -c hermes_check.f90 -o $@

$(BUILD)/hermes_check: $(OBJS) $(BUILD)/hermes_check.o
	$(FC) $(FFLAGS) -o $@ $(OBJS) $(BUILD)/hermes_check.o

check: $(BUILD)/hermes_check
	$(BUILD)/hermes_check

clean:
	rm -rf $(BUILD) *.mod
