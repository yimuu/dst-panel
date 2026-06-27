import { AppProviders } from './providers'
import { AppRouter } from './router'

export default function App() {
  return (
    <AppProviders>
      <span className="sr-only">饥荒联机版管理面板</span>
      <AppRouter />
    </AppProviders>
  )
}
