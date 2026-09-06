! Bounded command/event rings: arm, pair, take/retire slot (exclusive handles).
module hermes_rings
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: arm_command, arm_event, pair_rings, disarm, take_slot, retire_slot

  integer(i32), save :: next_id = 1000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'hermes_rings: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function arm_command(depth) result(ring)
    integer(i32), intent(in) :: depth
    if (depth <= 0) error stop 'arm_command: depth must be > 0'
    ring = mint()
  end function arm_command

  type(handle_t) function arm_event(depth) result(ring)
    integer(i32), intent(in) :: depth
    if (depth <= 0) error stop 'arm_event: depth must be > 0'
    ring = mint()
  end function arm_event

  type(handle_t) function pair_rings(command, event) result(transport)
    type(handle_t), intent(inout) :: command, event
    call kill(command)
    call kill(event)
    transport = mint()
  end function pair_rings

  subroutine disarm(transport)
    type(handle_t), intent(inout) :: transport
    call kill(transport)
  end subroutine disarm

  type(handle_t) function take_slot(transport) result(token)
    type(handle_t), intent(inout) :: transport
    call kill(transport)
    token = mint()
  end function take_slot

  type(handle_t) function retire_slot(token) result(transport)
    type(handle_t), intent(inout) :: token
    call kill(token)
    transport = mint()
  end function retire_slot

end module hermes_rings
