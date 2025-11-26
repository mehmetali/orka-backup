<?php

namespace App\Filament\Pages;

use App\Filament\Widgets\BackupChart;
use App\Filament\Widgets\LatestBackups;
use App\Filament\Widgets\StorageStats;
use Filament\Pages\Page;

class BackupDashboard extends Page
{
    protected static ?string $navigationIcon = 'heroicon-o-chart-bar';

    protected static string $view = 'filament.pages.backup-dashboard';

    protected function getHeaderWidgets(): array
    {
        return [
            StorageStats::class,
            BackupChart::class,
            LatestBackups::class,
        ];
    }
}
