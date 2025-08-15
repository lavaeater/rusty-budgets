/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["./*.{html,js}", "./templates/**/*.{html,js,tera}"],
    theme: {
        extend: {},
    },
    plugins: [
        require('@tailwindcss/typography'),
        require('daisyui'),
    ],
    daisyui: {
        themes: ["retro"]
    }
}

