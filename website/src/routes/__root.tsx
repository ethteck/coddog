import { createRootRoute, Link, Outlet } from '@tanstack/react-router';
import logo from '../assets/coddoglogo.svg';

export const Route = createRootRoute({
  head: () => ({
    meta: [
      {
        title: 'coddog',
      },
    ],
  }),
  component: () => (
    <div className="page-container">
      <div className="header">
        <Link to="/">
          <img className="logo" src={logo} alt="coddog logo" />
        </Link>{' '}
        <div className="topnav">
          <Link
            to="/search"
            search={{ name: '' }}
            className="[&.active]:font-bold"
          >
            Search
          </Link>{' '}
          <Link to="/about" className="[&.active]:font-bold">
            About
          </Link>{' '}
        </div>
      </div>
      <div className="content">
        <Outlet />
      </div>
      <div className="footer">
        <p>
          <a
            href={`https://github.com/ethteck/coddog/commit/${process.env.GIT_HASH}`}
            target="_blank"
            rel="noopener noreferrer"
            title={`Built on commit ${process.env.GIT_HASH}`}
            className="footer-link"
          >
            <img
              src={logo}
              width="60px"
              className="footer-logo"
              alt="coddog logo mini"
            />
          </a>
          , the dog that sniffs for cod
        </p>
        <p style={{ fontSize: '0.8rem', color: 'var(--color-text-muted)' }}>
          made with{' '}
          <span style={{ color: 'var(--color-primary-dark)' }}>cod</span> by{' '}
          <a href="https://github.com/ethteck">ethteck</a>
        </p>
      </div>
      {/*<TanStackRouterDevtools/>*/}
    </div>
  ),
});
