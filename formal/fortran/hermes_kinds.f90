! Hermes formal kinds — shared integers / phase codes.
module hermes_kinds
  implicit none
  private
  public :: i32, i64, phase_offline, phase_probed, phase_firmwared, &
            phase_queued, phase_negotiated, phase_online, phase_recovering, &
            phase_quarantined, phase_label

  integer, parameter :: i32 = selected_int_kind(9)
  integer, parameter :: i64 = selected_int_kind(15)

  integer(i32), parameter :: phase_offline = 0
  integer(i32), parameter :: phase_probed = 1
  integer(i32), parameter :: phase_firmwared = 2
  integer(i32), parameter :: phase_queued = 3
  integer(i32), parameter :: phase_negotiated = 4
  integer(i32), parameter :: phase_online = 5
  integer(i32), parameter :: phase_recovering = 6
  integer(i32), parameter :: phase_quarantined = 7

contains

  pure function phase_label(p) result(s)
    integer(i32), intent(in) :: p
    character(len=12) :: s
    select case (p)
    case (phase_offline); s = 'OFFLINE'
    case (phase_probed); s = 'PROBED'
    case (phase_firmwared); s = 'FIRMWARED'
    case (phase_queued); s = 'QUEUED'
    case (phase_negotiated); s = 'NEGOTIATED'
    case (phase_online); s = 'ONLINE'
    case (phase_recovering); s = 'RECOVERING'
    case (phase_quarantined); s = 'QUARANTINED'
    case default; s = 'UNKNOWN'
    end select
  end function phase_label

end module hermes_kinds
