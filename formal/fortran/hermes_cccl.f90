! CUDA/CCCL session — driver/context/buffer/stream exclusive handles.
module hermes_cccl
  use hermes_kinds, only: i32, i64
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: require_gsp_cuda, open_driver, create_context, alloc_device, &
            free_device, load_module, destroy_module, create_stream, &
            destroy_stream, close_context, close_driver

  integer(i32), save :: next_id = 9000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'cccl: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function require_gsp_cuda(gsp_online) result(g)
    logical, intent(in) :: gsp_online
    if (.not. gsp_online) error stop 'cuda: GSP offline'
    g = mint()
  end function require_gsp_cuda

  type(handle_t) function open_driver(g) result(d)
    type(handle_t), intent(inout) :: g
    call kill(g)
    d = mint()
  end function open_driver

  type(handle_t) function create_context(d) result(ctx)
    type(handle_t), intent(in) :: d
    if (.not. handle_is_live(d)) error stop 'create_context: dead driver'
    ctx = mint()
  end function create_context

  type(handle_t) function alloc_device(ctx, bytes) result(buf)
    type(handle_t), intent(in) :: ctx
    integer(i64), intent(in) :: bytes
    if (.not. handle_is_live(ctx)) error stop 'alloc_device: dead ctx'
    if (bytes <= 0) error stop 'alloc_device: zero size'
    buf = mint()
  end function alloc_device

  type(handle_t) function free_device(buf) result(ctx)
    type(handle_t), intent(inout) :: buf
    call kill(buf)
    ctx = mint()
  end function free_device

  type(handle_t) function load_module(ctx) result(m)
    type(handle_t), intent(in) :: ctx
    if (.not. handle_is_live(ctx)) error stop 'load_module: dead ctx'
    m = mint()
  end function load_module

  type(handle_t) function destroy_module(m) result(ctx)
    type(handle_t), intent(inout) :: m
    call kill(m)
    ctx = mint()
  end function destroy_module

  type(handle_t) function create_stream(ctx) result(s)
    type(handle_t), intent(in) :: ctx
    if (.not. handle_is_live(ctx)) error stop 'create_stream: dead ctx'
    s = mint()
  end function create_stream

  type(handle_t) function destroy_stream(s) result(ctx)
    type(handle_t), intent(inout) :: s
    call kill(s)
    ctx = mint()
  end function destroy_stream

  type(handle_t) function close_context(ctx) result(d)
    type(handle_t), intent(inout) :: ctx
    call kill(ctx)
    d = mint()
  end function close_context

  subroutine close_driver(d)
    type(handle_t), intent(inout) :: d
    call kill(d)
  end subroutine close_driver

end module hermes_cccl
