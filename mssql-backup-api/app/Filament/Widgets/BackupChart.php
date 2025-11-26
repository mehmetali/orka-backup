<?php

namespace App\Filament\Widgets;

use App\Models\Backup;
use Filament\Widgets\ChartWidget;
use Illuminate\Support\Carbon;

class BackupChart extends ChartWidget
{
    protected static ?string $heading = 'Backups in Last 30 Days';

    protected function getData(): array
    {
        $data = Backup::query()
            ->where('backup_completed_at', '>=', now()->subDays(30))
            ->select('status', \DB::raw('DATE(backup_completed_at) as date'), \DB::raw('count(*) as count'))
            ->groupBy('status', 'date')
            ->orderBy('date')
            ->get();

        $labels = [];
        for ($i = 29; $i >= 0; $i--) {
            $labels[] = Carbon::now()->subDays($i)->format('M d');
        }

        $datasets = [
            'success' => array_fill_keys($labels, 0),
            'failed' => array_fill_keys($labels, 0),
        ];

        foreach ($data as $item) {
            $dateLabel = Carbon::parse($item->date)->format('M d');
            if (isset($datasets[$item->status][$dateLabel])) {
                $datasets[$item->status][$dateLabel] = $item->count;
            }
        }

        return [
            'datasets' => [
                [
                    'label' => 'Successful Backups',
                    'data' => array_values($datasets['success']),
                    'borderColor' => 'rgb(75, 192, 192)',
                    'tension' => 0.1,
                ],
                [
                    'label' => 'Failed Backups',
                    'data' => array_values($datasets['failed']),
                    'borderColor' => 'rgb(255, 99, 132)',
                    'tension' => 0.1,
                ],
            ],
            'labels' => $labels,
        ];
    }

    protected function getType(): string
    {
        return 'line';
    }
}
