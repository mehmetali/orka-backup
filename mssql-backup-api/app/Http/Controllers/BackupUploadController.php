<?php

namespace App\Http\Controllers;

use App\Http\Requests\BackupUploadRequest;
use App\Models\Backup;
use Illuminate\Http\JsonResponse;
use Illuminate\Support\Facades\Storage;
use Illuminate\Support\Str;

class BackupUploadController extends Controller
{
    /**
     * Handle the incoming backup upload request.
     *
     * @param BackupUploadRequest $request
     * @return JsonResponse
     */
    public function upload(BackupUploadRequest $request): JsonResponse
    {
        $validated = $request->validated();
        $file = $request->file('backup_file');

        $server = \App\Models\Server::where('token', $validated['token'])->firstOrFail();

        // Generate a unique file path
        $serverName = Str::slug($server->name);
        $dbName = Str::slug($validated['database_name']);
        $timestamp = now()->format('Ymd_His');
        $originalName = pathinfo($file->getClientOriginalName(), PATHINFO_FILENAME);
        $fileName = "{$timestamp}_{$originalName}.bak";

        $filePath = $file->storeAs("backups/{$serverName}/{$dbName}", $fileName, 'local');

        $backup = Backup::create([
            'user_id' => auth()->id(),
            'server_id' => $server->id,
            'db_name' => $validated['database_name'],
            'file_path' => $filePath,
            'file_size_bytes' => $file->getSize(),
            'checksum_sha256' => $validated['checksum_sha256'],
            'backup_started_at' => $validated['backup_started_at'],
            'backup_completed_at' => $validated['backup_completed_at'],
            'duration_seconds' => $validated['duration_seconds'],
            'status' => 'success',
        ]);

        return response()->json([
            'status' => 'ok',
            'backup_id' => $backup->id,
        ]);
    }
}
