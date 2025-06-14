import ReactDOM from 'react-dom/client';
import './styles.css';
import React from 'react';
import App from './app.tsx';

const rootEl = document.getElementById('root');

if (rootEl) {
    const root = ReactDOM.createRoot(rootEl);
    root.render(
        <React.StrictMode>
            <App/>
        </React.StrictMode>,
    );
}
