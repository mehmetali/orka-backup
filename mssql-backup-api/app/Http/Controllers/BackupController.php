<?php

namespace App\Http\Controllers;

use App\Models\Backup;
use Illuminate\Http\Request;

use Illuminate\Support\Facades\Storage;
use Symfony\Component\HttpFoundation\StreamedResponse;
use Illuminate\Http\JsonResponse;
use Illuminate\Support\Facades\URL;

class BackupController extends Controller
{
    public function index(): JsonResponse
    {
        return response()->json(Backup::all());
    }

    public function download(Request $request, Backup $backup): JsonResponse
    {
        $user = auth()->user();

        if ($user->group_id !== $backup->server->group_id) {
            abort(403);
        }

        $temporaryUrl = URL::temporarySignedRoute(
            'backups.stream',
            now()->addMinutes(15),
            ['backup' => $backup->id]
        );

        return response()->json(['url' => $temporaryUrl]);
    }

    public function streamBackup(Request $request, Backup $backup): StreamedResponse
    {
        return Storage::disk('local')->download($backup->file_path);
    }
}
