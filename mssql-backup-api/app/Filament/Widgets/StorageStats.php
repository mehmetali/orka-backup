<?php

namespace App\Filament\Widgets;

use App\Models\Backup;
use Filament\Widgets\StatsOverviewWidget as BaseWidget;
use Filament\Widgets\StatsOverviewWidget\Stat;

class StorageStats extends BaseWidget
{
    protected function getStats(): array
    {
        $totalSize = Backup::sum('file_size_bytes');
        $totalSizeMb = $totalSize / (1024 * 1024);

        return [
            Stat::make('Total Storage Used', number_format($totalSizeMb, 2) . ' MB'),
        ];
    }
}
