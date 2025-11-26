<?php

namespace App\Http\Requests;

use Illuminate\Foundation\Http\FormRequest;

class BackupUploadRequest extends FormRequest
{
    /**
     * Determine if the user is authorized to make this request.
     */
    public function authorize(): bool
    {
        return true;
    }

    /**
     * Get the validation rules that apply to the request.
     *
     * @return array<string, \Illuminate\Contracts\Validation\ValidationRule|array<mixed>|string>
     */
    public function rules(): array
    {
        return [
            'server_name' => 'required|string|max:255',
            'database_name' => 'required|string|max:255',
            'backup_started_at' => 'required|date',
            'backup_completed_at' => 'required|date|after_or_equal:backup_started_at',
            'duration_seconds' => 'required|integer|min:0',
            'checksum_sha256' => 'required|string|size:64',
            'backup_file' => 'required|file',
        ];
    }
}
