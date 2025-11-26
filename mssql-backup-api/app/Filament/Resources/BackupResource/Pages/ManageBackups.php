<?php

namespace App\Filament\Resources\BackupResource\Pages;

use App\Filament\Resources\BackupResource;
use Filament\Actions;
use Filament\Resources\Pages\ManageRecords;

class ManageBackups extends ManageRecords
{
    protected static string $resource = BackupResource::class;

    protected function getHeaderActions(): array
    {
        return [
            Actions\CreateAction::make(),
        ];
    }
}
