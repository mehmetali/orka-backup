<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Factories\HasFactory;
use Illuminate\Database\Eloquent\Model;

class Server extends Model
{
    use HasFactory;

    protected $fillable = [
        'name',
        'host',
        'token',
        'group_id',
    ];

    public function group()
    {
        return $this->belongsTo(Group::class);
    }
}
