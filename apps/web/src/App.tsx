import { lazy, Suspense, type ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { AppShell } from './components/shell'
import { ApiSettingsProvider } from './lib/api'

const RecordsPage = lazy(() =>
  import('./pages/records-page').then((module) => ({ default: module.RecordsPage })),
)
const GraphPage = lazy(() =>
  import('./pages/graph-page').then((module) => ({ default: module.GraphPage })),
)
const OperationsPage = lazy(() =>
  import('./pages/operations-page').then((module) => ({ default: module.OperationsPage })),
)
const AuditPage = lazy(() =>
  import('./pages/audit-page').then((module) => ({ default: module.AuditPage })),
)

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 15_000,
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
})

function App() {
  return (
    <ApiSettingsProvider>
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <Routes>
            <Route element={<AppShell />}>
              <Route index element={<Navigate to="/records" replace />} />
              <Route path="/records" element={<RouteLoader><RecordsPage /></RouteLoader>} />
              <Route path="/graph" element={<RouteLoader><GraphPage /></RouteLoader>} />
              <Route path="/operations" element={<RouteLoader><OperationsPage /></RouteLoader>} />
              <Route path="/audit" element={<RouteLoader><AuditPage /></RouteLoader>} />
            </Route>
          </Routes>
        </BrowserRouter>
      </QueryClientProvider>
    </ApiSettingsProvider>
  )
}

function RouteLoader({ children }: { children: ReactNode }) {
  return (
    <Suspense
      fallback={
        <div className="rounded-[var(--radius)] border border-[var(--border)] bg-[color:var(--bg-panel)] px-6 py-16 text-center text-sm text-[var(--text-muted)]">
          Loading operator surface...
        </div>
      }
    >
      {children}
    </Suspense>
  )
}

export default App
