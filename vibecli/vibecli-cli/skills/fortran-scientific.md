---
triggers: ["Fortran", "Fortran 90", "Fortran 2008", "Fortran 2018", "gfortran", "ifort", "HPC Fortran", "numerical Fortran", "Fortran array"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["gfortran"]
category: fortran
---

# Fortran

When writing modern Fortran (Fortran 2008/2018) for scientific and HPC computing:

1. Use free-form source (`.f90`/`.f03`/`.f08` extensions) — not fixed-form (`.f`/`.f77`); use `implicit none` in every program unit to require explicit type declarations; catches typos that silently create wrong-typed variables.
2. Use modules for code organization: `module physics; implicit none; contains; function kinetic_energy(m, v) result(ke) ... end function; end module` — `use physics, only: kinetic_energy` for explicit imports.
3. Leverage array operations (Fortran's strength): `A = B + C` operates element-wise on entire arrays; `where (A > 0) B = sqrt(A)` for conditional assignment; `sum(A, dim=1)` for column sums — avoid explicit loops when array syntax suffices.
4. Use `allocatable` arrays for dynamic sizing: `real, allocatable :: data(:,:); allocate(data(n, m))` — Fortran manages deallocation automatically when allocatables go out of scope; use `move_alloc` for efficient ownership transfer.
5. Use `intent` attributes on all procedure arguments: `subroutine solve(A, b, x) real, intent(in) :: A(:,:), b(:); real, intent(out) :: x(:)` — `in` = read-only, `out` = write-only, `inout` = read-write; enables compiler optimizations and catches misuse.
6. Use derived types for structured data: `type :: particle; real :: pos(3), vel(3), mass; end type` — with type-bound procedures: `type :: particle; contains; procedure :: update => update_particle; end type` for OOP.
7. Parallelize with coarrays (Fortran 2008): `real :: local_sum; real :: global_sum[*]; global_sum[this_image()] = local_sum; sync all` — native parallel feature; compile with `gfortran -fcoarray=lib` or use Intel Fortran for multi-image support.
8. Use OpenMP for shared-memory parallelism: `!$omp parallel do reduction(+:total) do i = 1, n; total = total + a(i); end do; !$omp end parallel do` — compile with `-fopenmp`; Fortran's array semantics make parallelization natural.
9. Call BLAS/LAPACK for linear algebra: `call dgemm('N','N',m,n,k,1.0d0,A,m,B,k,0.0d0,C,m)` for matrix multiply; `call dgesv(n,1,A,n,ipiv,b,n,info)` for solving Ax=b — these are highly optimized (MKL, OpenBLAS).
10. Use `iso_fortran_env` for portable types: `use iso_fortran_env, only: real64, int32, int64` — ensures consistent precision across compilers/platforms; `real(real64) :: x` is always 64-bit IEEE double.
11. Interface with C using `iso_c_binding`: `interface; function c_func(x) bind(C, name='c_func'); use iso_c_binding; real(c_double), value :: x; real(c_double) :: c_func; end function; end interface` — enables calling C libraries and being called from C.
12. Compile with modern flags: `gfortran -O2 -Wall -Wextra -Wconversion -fcheck=all -fbacktrace` for development; remove `-fcheck=all` for production (it adds runtime bounds checking) — use `-march=native` for target-specific optimization.
