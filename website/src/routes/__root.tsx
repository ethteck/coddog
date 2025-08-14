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
    <>
      <div className="header">
        <Link to="/">
          <img className="logo" src={logo} alt="coddog logo" />
        </Link>{' '}
        <div className="topnav">
          <Link
            to="/symbol"
            search={{ name: '' }}
            className="[&.active]:font-bold"
          >
            Symbol
          </Link>{' '}
          <Link to="/admin" className="[&.active]:font-bold">
            Admin
          </Link>{' '}
        </div>
      </div>
      <div className="content">
        <Outlet />
      </div>
      <div className="footer" style={{ display: 'flex', alignItems: 'center' }}>
        <a
          href={`https://github.com/ethteck/coddog/commit/${process.env.GIT_HASH || 'unknown'}`}
          target="_blank"
          rel="noopener noreferrer"
        >
          <img
            src={logo}
            width="60px"
            style={{
              filter: 'grayscale(1) contrast(200%) brightness(0.5)',
              marginRight: '5px',
            }}
            alt="coddog logo mini"
          />
        </a>
        <p>, the dog that sniffs for cod</p>
      </div>
      {/*<TanStackRouterDevtools/>*/}
    </>
  ),
});
