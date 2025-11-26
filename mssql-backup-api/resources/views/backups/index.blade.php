<!DOCTYPE html>
<html>
<head>
    <title>Backups</title>
</head>
<body>
    <h1>Backups</h1>
    <table>
        <thead>
            <tr>
                <th>Server</th>
                <th>Database</th>
                <th>Date</th>
                <th>Size</th>
                <th>Status</th>
                <th>Actions</th>
            </tr>
        </thead>
        <tbody>
            @foreach ($backups as $backup)
                <tr>
                    <td>{{ $backup->server_name }}</td>
                    <td>{{ $backup->db_name }}</td>
                    <td>{{ $backup->created_at->format('Y-m-d H:i:s') }}</td>
                    <td>{{ $backup->file_size_bytes }}</td>
                    <td>{{ $backup->status }}</td>
                    <td>
                        <a href="{{ App\Http\Controllers\BackupController::sign($backup) }}">Download</a>
                    </td>
                </tr>
            @endforeach
        </tbody>
    </table>
</body>
</html>
