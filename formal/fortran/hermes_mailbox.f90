! Falcon mailbox session — HELLO post consumes the session token.
module hermes_mailbox
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: require_gsp_mb, open_mailbox, post_hello, observe_ready_resp

  integer(i32), save :: next_id = 6000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'mailbox: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function require_gsp_mb(gsp_online) result(g)
    logical, intent(in) :: gsp_online
    if (.not. gsp_online) error stop 'mailbox: GSP offline'
    g = mint()
  end function require_gsp_mb

  type(handle_t) function open_mailbox(g) result(m)
    type(handle_t), intent(inout) :: g
    call kill(g)
    m = mint()
  end function open_mailbox

  type(handle_t) function post_hello(m) result(resp)
    type(handle_t), intent(inout) :: m
    call kill(m)
    resp = mint()
  end function post_hello

  pure logical function observe_ready_resp(r) result(ok)
    type(handle_t), intent(in) :: r
    ok = handle_is_live(r)
  end function observe_ready_resp

end module hermes_mailbox
