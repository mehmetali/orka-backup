<?php

use App\Http\Controllers\BackupController;
use Illuminate\Support\Facades\Route;

Route::get('/', function () {
    return view('welcome');
});

Route::get('/backups/{backup}/stream', [BackupController::class, 'streamBackup'])
    ->name('backups.stream')
    ->middleware('signed');
