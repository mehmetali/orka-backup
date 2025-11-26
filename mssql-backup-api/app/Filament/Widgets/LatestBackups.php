<?php

namespace App\Filament\Widgets;

use App\Filament\Resources\BackupResource;
use Filament\Tables;
use Filament\Tables\Table;
use Filament\Widgets\TableWidget as BaseWidget;

class LatestBackups extends BaseWidget
{
    protected static ?int $sort = 3;

    public function table(Table $table): Table
    {
        return $table
            ->query(BackupResource::getEloquentQuery()->latest('backup_completed_at')->limit(10))
            ->columns([
                Tables\Columns\TextColumn::make('server_name'),
                Tables\Columns\TextColumn::make('db_name'),
                Tables\Columns\TextColumn::make('backup_completed_at')->dateTime(),
                Tables\Columns\BadgeColumn::make('status')
                    ->colors([
                        'success' => 'success',
                        'danger' => 'failed',
                    ]),
            ]);
    }
}
