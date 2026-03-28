import {
  type ColumnFiltersState,
  type ColumnDef,
  flexRender,
  getFilteredRowModel,
  getCoreRowModel,
  getSortedRowModel,
  type SortingState,
  useReactTable,
} from '@tanstack/react-table'
import clsx from 'clsx'
import { ArrowDownWideNarrow, ArrowUpWideNarrow } from 'lucide-react'
import { useMemo, useRef, useState } from 'react'

export type DataTableColumnMeta = {
  filterVariant?: 'text' | 'select'
  filterPlaceholder?: string
  filterOptions?: string[]
  sticky?: 'left' | 'right'
}

export function DataTable<T>({
  data,
  columns,
  getRowId,
  onRowClick,
  selectedRowId,
  emptyState,
}: {
  data: T[]
  columns: ColumnDef<T, unknown>[]
  getRowId: (row: T) => string
  onRowClick?: (row: T) => void
  selectedRowId?: string
  emptyState?: string
}) {
  const [sorting, setSorting] = useState<SortingState>([])
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([])
  const [scrollTop, setScrollTop] = useState(0)
  const scrollRef = useRef<HTMLDivElement>(null)

  // eslint-disable-next-line react-hooks/incompatible-library
  const table = useReactTable({
    data,
    columns,
    getRowId,
    state: { sorting, columnFilters },
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
  })

  const rows = table.getRowModel().rows
  const rowHeight = 56
  const overscan = 8
  const viewportHeight = 640
  const startIndex = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan)
  const endIndex = Math.min(rows.length, Math.ceil((scrollTop + viewportHeight) / rowHeight) + overscan)
  const visibleRows = useMemo(() => rows.slice(startIndex, endIndex), [endIndex, rows, startIndex])
  const topSpacerHeight = startIndex * rowHeight
  const bottomSpacerHeight = Math.max(0, (rows.length - endIndex) * rowHeight)
  const hasColumnFilters = table
    .getVisibleLeafColumns()
    .some((column) => getMeta(column.columnDef)?.filterVariant)

  function getMeta(column: ColumnDef<T, unknown>): DataTableColumnMeta | undefined {
    return column.meta as DataTableColumnMeta | undefined
  }

  function getStickyClass(sticky?: 'left' | 'right', tone: 'header' | 'body' = 'body') {
    const bg = tone === 'header' ? 'bg-[color:var(--bg-panel-2)]' : 'bg-[color:var(--bg-panel)]'
    if (sticky === 'left') {
      return `sticky left-0 z-[2] ${bg} shadow-[12px_0_18px_rgba(0,0,0,0.14)]`
    }
    if (sticky === 'right') {
      return `sticky right-0 z-[2] ${bg} shadow-[-12px_0_18px_rgba(0,0,0,0.14)]`
    }
    return ''
  }

  return (
    <div className="overflow-hidden rounded-[10px] border border-[var(--border)]">
      <div
        ref={scrollRef}
        className="max-h-[640px] overflow-auto"
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      >
        <table className="min-w-full border-collapse text-left text-sm">
          <thead className="sticky top-0 z-10 bg-[color:var(--bg-panel-2)]">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => {
                  const meta = getMeta(header.column.columnDef)
                  return (
                  <th
                    key={header.id}
                    className={clsx(
                      'border-b border-[var(--border)] px-3 py-2 text-[9px] uppercase tracking-[0.2em] text-[var(--text-muted)]',
                      getStickyClass(meta?.sticky, 'header'),
                    )}
                  >
                    {header.isPlaceholder ? null : (
                      <button
                        className={clsx(
                          'flex items-center gap-2 text-left',
                          header.column.getCanSort() ? 'cursor-pointer' : 'cursor-default',
                        )}
                        onClick={header.column.getToggleSortingHandler()}
                        type="button"
                      >
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        {{
                          asc: <ArrowUpWideNarrow className="h-3.5 w-3.5" />,
                          desc: <ArrowDownWideNarrow className="h-3.5 w-3.5" />,
                        }[header.column.getIsSorted() as string] ?? null}
                      </button>
                    )}
                  </th>
                  )
                })}
              </tr>
            ))}
            {hasColumnFilters ? (
              <tr>
                {table.getVisibleLeafColumns().map((column) => {
                  const meta = getMeta(column.columnDef)
                  const filterValue = column.getFilterValue()
                  return (
                    <th
                      key={`${column.id}-filter`}
                      className={clsx(
                        'border-b border-[var(--border)]/80 px-3 py-2',
                        getStickyClass(meta?.sticky, 'header'),
                      )}
                    >
                      {meta?.filterVariant === 'text' ? (
                        <input
                          className="w-full rounded-md border border-[var(--border)] bg-[color:var(--bg)] px-2 py-1 text-[11px] font-normal normal-case tracking-normal text-[var(--text-strong)] outline-none placeholder:text-[var(--text-dim)] focus:border-[var(--border-strong)]"
                          onChange={(event) => column.setFilterValue(event.target.value)}
                          placeholder={meta.filterPlaceholder ?? 'Filter'}
                          value={(filterValue ?? '') as string}
                        />
                      ) : meta?.filterVariant === 'select' ? (
                        <select
                          className="w-full rounded-md border border-[var(--border)] bg-[color:var(--bg)] px-2 py-1 text-[11px] font-normal normal-case tracking-normal text-[var(--text-strong)] outline-none focus:border-[var(--border-strong)]"
                          onChange={(event) => column.setFilterValue(event.target.value || undefined)}
                          value={(filterValue ?? '') as string}
                        >
                          <option value="">All</option>
                          {(meta.filterOptions ?? []).map((option) => (
                            <option key={option} value={option}>
                              {option}
                            </option>
                          ))}
                        </select>
                      ) : null}
                    </th>
                  )
                })}
              </tr>
            ) : null}
          </thead>
          <tbody>
            {rows.length > 0 ? (
              <>
                {topSpacerHeight > 0 ? (
                  <tr aria-hidden="true">
                    <td colSpan={columns.length} style={{ height: `${topSpacerHeight}px`, padding: 0 }} />
                  </tr>
                ) : null}
                {visibleRows.map((row) => (
                  <tr
                    key={row.id}
                    className={clsx(
                      'border-b border-[var(--border)]/70 transition-colors last:border-b-0',
                      onRowClick ? 'cursor-pointer hover:bg-[var(--interactive-hover)]' : '',
                      selectedRowId === row.id ? 'bg-[rgba(255,106,193,0.1)]' : '',
                    )}
                    onClick={() => onRowClick?.(row.original)}
                  >
                    {row.getVisibleCells().map((cell) => {
                      const meta = getMeta(cell.column.columnDef)
                      return (
                        <td
                          key={cell.id}
                          className={clsx(
                            'px-3 py-2 align-top text-[13px] text-[var(--text)]',
                            getStickyClass(meta?.sticky),
                          )}
                        >
                          {flexRender(cell.column.columnDef.cell, cell.getContext())}
                        </td>
                      )
                    })}
                  </tr>
                ))}
                {bottomSpacerHeight > 0 ? (
                  <tr aria-hidden="true">
                    <td colSpan={columns.length} style={{ height: `${bottomSpacerHeight}px`, padding: 0 }} />
                  </tr>
                ) : null}
              </>
            ) : (
              <tr>
                <td
                  className="px-4 py-12 text-center text-sm text-[var(--text-muted)]"
                  colSpan={columns.length}
                >
                  {emptyState ?? 'No data available.'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
