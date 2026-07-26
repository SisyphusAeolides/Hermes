! DRM/KMS modeset session — atomic apply requires Online GSP token.
module hermes_drm_kms
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: require_gsp_drm, open_crtc, open_plane, bind_framebuffer, &
            atomic_apply, disable_crtc, close_modeset

  integer(i32), save :: next_id = 8000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'drm_kms: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function require_gsp_drm(gsp_online) result(g)
    logical, intent(in) :: gsp_online
    if (.not. gsp_online) error stop 'drm: GSP offline'
    g = mint()
  end function require_gsp_drm

  type(handle_t) function open_crtc(g) result(c)
    type(handle_t), intent(in) :: g
    if (.not. handle_is_live(g)) error stop 'open_crtc: dead gsp'
    c = mint()
  end function open_crtc

  type(handle_t) function open_plane(g) result(p)
    type(handle_t), intent(in) :: g
    if (.not. handle_is_live(g)) error stop 'open_plane: dead gsp'
    p = mint()
  end function open_plane

  type(handle_t) function bind_framebuffer(p, fb_id) result(out)
    type(handle_t), intent(inout) :: p
    integer(i32), intent(in) :: fb_id
    if (fb_id <= 0) error stop 'bind_framebuffer: invalid fb'
    call kill(p)
    out = mint()
  end function bind_framebuffer

  type(handle_t) function atomic_apply(c, p, g) result(modeset)
    type(handle_t), intent(inout) :: c, p, g
    call kill(c)
    call kill(p)
    call kill(g)
    modeset = mint()
  end function atomic_apply

  type(handle_t) function disable_crtc(m) result(g)
    type(handle_t), intent(inout) :: m
    call kill(m)
    g = mint()
  end function disable_crtc

  subroutine close_modeset(m)
    type(handle_t), intent(inout) :: m
    call kill(m)
  end subroutine close_modeset

end module hermes_drm_kms
