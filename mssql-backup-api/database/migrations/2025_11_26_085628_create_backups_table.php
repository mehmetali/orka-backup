<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    /**
     * Run the migrations.
     */
    public function up(): void
    {
        Schema::create('backups', function (Blueprint $table) {
            $table->id();
            $table->unsignedBigInteger('server_id');
            $table->string('db_name');
            $table->string('file_path');
            $table->unsignedBigInteger('file_size_bytes');
            $table->string('checksum_sha256', 64);
            $table->timestamp('backup_started_at');
            $table->timestamp('backup_completed_at');
            $table->unsignedInteger('duration_seconds');
            $table->string('status')->default('success'); // success | failed
            $table->timestamps();
        });
    }

    /**
     * Reverse the migrations.
     */
    public function down(): void
    {
        Schema::dropIfExists('backups');
    }
};
