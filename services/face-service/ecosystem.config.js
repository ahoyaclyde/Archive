// ecosystem.config.js — PM2 process configuration (Render-ready)
module.exports = {
    apps: [
        {
            name:       'face-service',
            script:     'face_service.js',

            // Relative path — works on any machine, any deploy environment.
            // PM2 resolves this relative to where `pm2 start` is called from,
            // which is services/face-service/ in start.sh.
            cwd: './',

            instances:  1,
            exec_mode:  'fork',

            autorestart:   true,
            restart_delay: 3000,
            max_restarts:  10,

            env: {
                NODE_ENV:          'production',
                FACE_SERVICE_PORT: '3001',
                FACE_SERVICE_HOST: '127.0.0.1',
                FACE_MODELS_PATH:  './models',   // relative to cwd above
                FACE_THRESHOLD:    '0.55',
                FACE_MAX_DIM:      '640',
            },

            out_file:        './logs/face-service-out.log',
            error_file:      './logs/face-service-err.log',
            log_date_format: 'YYYY-MM-DD HH:mm:ss',

            kill_timeout:   10000,
            wait_ready:     true,
            listen_timeout: 15000,
        },
    ],
};