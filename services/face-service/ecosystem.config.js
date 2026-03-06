// ecosystem.config.js — PM2 process configuration
module.exports = {
    apps: [
        {
            name:       'face-service',
            script:     'face_service.js',

            // Absolute working directory — PM2 always starts from here
            // regardless of where the `pm2 start` command was run from.
            cwd: '/media/denso/9fab6735-657f-4583-a14b-321dee685e66/denso/MasterX/WORK/Barrell/Flug/Edition2-NFL/CrimeBank/services/face-service',

            instances:  1,
            exec_mode:  'fork',

            autorestart:   true,
            restart_delay: 3000,
            max_restarts:  10,

           env: {
                NODE_ENV:          'production',
                FACE_SERVICE_PORT: '3001',
                FACE_SERVICE_HOST: '127.0.0.1',
                FACE_MODELS_PATH:  '/media/denso/9fab6735-657f-4583-a14b-321dee685e66/denso/MasterX/WORK/Barrell/Flug/Edition2-NFL/CrimeBank/services/face-service/models',
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