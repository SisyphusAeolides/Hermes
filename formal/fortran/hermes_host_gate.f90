! Host facts → exclusive Online authority (mint only when gates pass).
module hermes_host_gate
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: host_facts_t, observe_facts, isolation_ready, may_claim_online, &
            mint_authority, drop_authority

  type :: host_facts_t
    logical :: iommu = .false.
    logical :: nouveau_bound = .false.
    logical :: bar_mapped = .false.
    logical :: firmware_present = .false.
  end type host_facts_t

  integer(i32), save :: next_id = 7000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'host_gate: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  pure type(host_facts_t) function observe_facts(iommu, nouveau, bar, fw) result(f)
    logical, intent(in) :: iommu, nouveau, bar, fw
    f%iommu = iommu
    f%nouveau_bound = nouveau
    f%bar_mapped = bar
    f%firmware_present = fw
  end function observe_facts

  pure logical function isolation_ready(f) result(ok)
    type(host_facts_t), intent(in) :: f
    ok = f%iommu .and. .not. f%nouveau_bound
  end function isolation_ready

  pure logical function may_claim_online(f) result(ok)
    type(host_facts_t), intent(in) :: f
    ok = isolation_ready(f) .and. f%bar_mapped .and. f%firmware_present
  end function may_claim_online

  type(handle_t) function mint_authority(f) result(a)
    type(host_facts_t), intent(in) :: f
    if (.not. may_claim_online(f)) error stop 'mint_authority: gates incomplete'
    a = mint()
  end function mint_authority

  subroutine drop_authority(a)
    type(handle_t), intent(inout) :: a
    call kill(a)
  end subroutine drop_authority

end module hermes_host_gate
