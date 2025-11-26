<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

class Backup extends Model
{
    use HasFactory;

    public function user()
    {
        return $this->belongsTo(User::class);
    }

    public function server()
    {
        return $this->belongsTo(Server::class);
    }

    /**
     * The attributes that are mass assignable.
     *
     * @var array<int, string>
     */
    protected $fillable = [
        'user_id',
        'server_id',
        'db_name',
        'file_path',
        'file_size_bytes',
        'checksum_sha256',
        'backup_started_at',
        'backup_completed_at',
        'duration_seconds',
        'status',
    ];

    /**
     * The attributes that should be cast.
     *
     * @var array<string, string>
     */
    protected $casts = [
        'backup_started_at' => 'datetime',
        'backup_completed_at' => 'datetime',
        'file_size_bytes' => 'integer',
        'duration_seconds' => 'integer',
    ];
}
