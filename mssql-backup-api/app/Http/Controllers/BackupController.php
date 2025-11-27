<?php

namespace App\Http\Controllers;

use App\Models\Backup;
use Illuminate\Http\Request;

use Illuminate\Support\Facades\Storage;
use Symfony\Component\HttpFoundation\StreamedResponse;

class BackupController extends Controller
{
    public function download(Request $request, Backup $backup): StreamedResponse
    {
        $user = auth()->user();

        if ($user->group_id !== $backup->server->group_id) {
            abort(403);
        }

        return Storage::disk('local')->download($backup->file_path);
    }
}
