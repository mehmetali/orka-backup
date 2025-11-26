<?php

use App\Http\Controllers\BackupUploadController;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Route;

Route::middleware(['auth:sanctum'])->group(function () {
    Route::get('/user', function (Request $request) {
        return $request->user();
    });

    Route::post('/backups/upload', [BackupUploadController::class, 'upload']);
});
