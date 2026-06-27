import { Outlet } from 'react-router'

export default function AuthLayout() {
  return (
    <main className="auth-layout">
      <Outlet />
    </main>
  )
}
