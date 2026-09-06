/* Hermes NVIDIA drop-in surface names (C clients). */
#ifndef HERMES_NVIDIA_COMPAT_H
#define HERMES_NVIDIA_COMPAT_H

#define HERMES_MODULE_NVIDIA "nvidia"
#define HERMES_MODULE_NVIDIA_MODESET "nvidia-modeset"
#define HERMES_MODULE_NVIDIA_UVM "nvidia-uvm"
#define HERMES_MODULE_NVIDIA_DRM "nvidia-drm"
#define HERMES_MODULE_NVIDIA_PEERMEM "nvidia-peermem"

#define HERMES_DEV_NVIDIACTL "/dev/nvidiactl"
#define HERMES_DEV_NVIDIA0 "/dev/nvidia0"
#define HERMES_DEV_NVIDIA_UVM "/dev/nvidia-uvm"
#define HERMES_DEV_NVIDIA_UVM_TOOLS "/dev/nvidia-uvm-tools"
#define HERMES_DEV_NVIDIA_MODESET "/dev/nvidia-modeset"
#define HERMES_DEV_NVIDIA_DRM "/dev/nvidia-drm"
#define HERMES_DEV_NVIDIA_CAPS "/dev/nvidia-caps"

#define HERMES_BIN_NVIDIA_SMI "nvidia-smi"
#define HERMES_BIN_NVIDIA_SETTINGS "nvidia-settings"
#define HERMES_BIN_NVIDIA_MODPROBE "nvidia-modprobe"
#define HERMES_BIN_NVIDIA_PERSISTENCED "nvidia-persistenced"
#define HERMES_BIN_NVIDIA_CUDA_MPS_CONTROL "nvidia-cuda-mps-control"
#define HERMES_BIN_NVIDIA_DEBUGDUMP "nvidia-debugdump"

#define HERMES_LIB_NVIDIA_ML "libnvidia-ml.so.1"
#define HERMES_LIB_CUDA "libcuda.so.1"
#define HERMES_LIB_CUDART "libcudart.so.12"
#define HERMES_LIB_GLX_NVIDIA "libGLX_nvidia.so.0"
#define HERMES_LIB_EGL_NVIDIA "libEGL_nvidia.so.0"
#define HERMES_LIB_NVIDIA_CFG "libnvidia-cfg.so.1"

#endif
