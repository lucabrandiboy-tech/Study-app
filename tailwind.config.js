/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        navy: '#0B1026', deep: '#1A0F3A', card: '#141B3D', card2: '#241552',
        edge: '#7FD3FF', accent: '#A970FF', ink: '#E8ECFF', muted: '#A9A8D6',
        good: '#4ADE80', bad: '#F87171', streak: '#FF9F43',
      },
      fontFamily: { sans: ['Nunito', 'Inter', 'system-ui', 'sans-serif'] },
      keyframes: {
        pop: { '0%': { transform: 'scale(.6)', opacity: 0 }, '60%': { transform: 'scale(1.08)', opacity: 1 }, '100%': { transform: 'scale(1)' } },
        fall: { '0%': { transform: 'translateY(-10vh) rotate(0)' }, '100%': { transform: 'translateY(110vh) rotate(720deg)' } },
        flicker: { '0%,100%': { transform: 'scale(1)' }, '50%': { transform: 'scale(1.1) rotate(-3deg)' } },
      },
      animation: { pop: 'pop .45s ease-out', fall: 'fall 2.6s linear forwards', flicker: 'flicker 1.4s ease-in-out infinite' },
    },
  },
  plugins: [],
};
