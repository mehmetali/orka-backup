<?php

namespace App\Http\Controllers;

use App\Models\Backup;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\URL;
use Illuminate\Support\Facades\Storage;

class BackupController extends Controller
{
    public function index()
    {
        $backups = auth()->user()->backups()->latest()->paginate(10);

        return view('backups.index', compact('backups'));
    }

    public function download(Request $request, Backup $backup)
    {
        if (! $request->hasValidSignature()) {
            abort(401);
        }

        return Storage::download($backup->file_path);
    }

    public static function sign(Backup $backup)
    {
        return URL::temporarySignedRoute(
            'backups.download',
            now()->addMinutes(30),
            ['backup' => $backup->id]
        );
    }

    public static function verify(Request $request)
    {
        return $request->hasValidSignature();
    }
}
