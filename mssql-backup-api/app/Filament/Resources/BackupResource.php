<?php

namespace App\Filament\Resources;

use App\Filament\Resources\BackupResource\Pages;
use App\Models\Backup;
use Filament\Forms;
use Filament\Forms\Form;
use Filament\Resources\Resource;
use Filament\Tables;
use Filament\Tables\Table;
use Illuminate\Database\Eloquent\Builder;
use Illuminate\Database\Eloquent\SoftDeletingScope;
use Illuminate\Support\Facades\Storage;

class BackupResource extends Resource
{
    protected static ?string $model = Backup::class;

    protected static ?string $navigationIcon = 'heroicon-o-rectangle-stack';

    public static function form(Form $form): Form
    {
        return $form
            ->schema([
                // We don't need a form for creating/editing backups from the panel.
            ]);
    }

    public static function table(Table $table): Table
    {
        return $table
            ->columns([
                Tables\Columns\TextColumn::make('server_name')
                    ->searchable()
                    ->sortable(),
                Tables\Columns\TextColumn::make('db_name')
                    ->searchable()
                    ->sortable(),
                Tables\Columns\TextColumn::make('backup_completed_at')
                    ->dateTime()
                    ->sortable(),
                Tables\Columns\BadgeColumn::make('status')
                    ->colors([
                        'success' => 'success',
                        'danger' => 'failed',
                    ]),
                Tables\Columns\TextColumn::make('file_size_bytes')
                    ->label('File Size (MB)')
                    ->formatStateUsing(fn (string $state): string => number_format($state / (1024 * 1024), 2))
                    ->sortable(),
            ])
            ->filters([
                Tables\Filters\Filter::make('backup_completed_at')
                    ->form([
                        Forms\Components\DatePicker::make('completed_from'),
                        Forms\Components\DatePicker::make('completed_until'),
                    ])
                    ->query(function (Builder $query, array $data): Builder {
                        return $query
                            ->when(
                                $data['completed_from'],
                                fn (Builder $query, $date): Builder => $query->whereDate('backup_completed_at', '>=', $date),
                            )
                            ->when(
                                $data['completed_until'],
                                fn (Builder $query, $date): Builder => $query->whereDate('backup_completed_at', '<=', $date),
                            );
                    }),
                 Tables\Filters\SelectFilter::make('server_name')
                    ->options(fn () => Backup::distinct()->pluck('server_name', 'server_name')->all()),
                 Tables\Filters\SelectFilter::make('db_name')
                    ->options(fn () => Backup::distinct()->pluck('db_name', 'db_name')->all()),
            ])
            ->actions([
                Tables\Actions\Action::make('download')
                    ->label('Download')
                    ->icon('heroicon-o-arrow-down-tray')
                    ->url(fn (Backup $record) => Storage::disk('local')->url($record->file_path))
                    ->openUrlInNewTab(),
            ])
            ->bulkActions([
                //
            ]);
    }

    public static function getEloquentQuery(): Builder
    {
        return parent::getEloquentQuery()
            ->join('servers', 'backups.server_id', '=', 'servers.id')
            ->select('backups.*', 'servers.name as server_name');
    }

    public static function getPages(): array
    {
        return [
            'index' => Pages\ListBackups::route('/'),
        ];
    }

    public static function canCreate(): bool
    {
        return false;
    }
}
