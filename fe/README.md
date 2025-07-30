# R Data Core Admin Frontend

A Vue 3 + Vuetify 3 admin interface for the R Data Core dynamic entity management system.

## Tech Stack

- **Vue 3** (Composition API) - Progressive JavaScript framework
- **Vuetify 3** - Material Design component framework  
- **TypeScript** - Type safety and better developer experience
- **Vite** - Fast build tool with excellent tree-shaking
- **Pinia** - State management for Vue 3
- **Vue Router 4** - Client-side routing
- **Native Fetch** - HTTP client (zero external dependencies)

## Features

- 🔐 **Authentication System** - JWT-based login with route guards
- 📊 **Dashboard** - System metrics and quick actions
- 🗂️ **Class Definitions** - Manage dynamic entity schemas
- 🏗️ **Dynamic Entities** - CRUD operations with dynamic forms
- 🔑 **API Keys** - Generate and manage API keys
- 👥 **Permissions** - User access control
- ⚙️ **System Admin** - Configuration and maintenance

## Development

### Local Development
```bash
npm install
npm run dev
```

### Using Docker
The frontend is configured to run in Docker via the main compose.yaml:

```bash
# From the project root
docker compose up -d node
```

The application will be available at:
- **Local**: http://localhost:3000
- **Docker**: http://admin.rdatacore.docker

### API Integration

The frontend is configured to proxy API requests to the backend:
- `/api/*` → `http://rdatacore.docker`
- `/admin/api/*` → `http://rdatacore.docker`

## Project Structure

```
fe/
├── src/
│   ├── api/                    # HTTP client and API services
│   ├── components/             # Reusable Vue components
│   ├── layouts/                # Layout components
│   ├── pages/                  # Page components (routes)
│   ├── stores/                 # Pinia stores
│   ├── router/                 # Vue Router configuration
│   ├── types/                  # TypeScript type definitions
│   └── utils/                  # Utility functions
├── public/                     # Static assets
└── package.json               # Dependencies and scripts
```

## Available Scripts

- `npm run dev` - Start development server
- `npm run build` - Build for production
- `npm run preview` - Preview production build
- `npm run lint` - Run ESLint
- `npm run format` - Format code with Prettier

## Authentication

The app includes JWT-based authentication with:
- Login/logout functionality
- Route guards for protected pages
- Automatic token management
- Session persistence

## Custom HTTP Client

Instead of using Axios, we use a custom fetch wrapper that:
- Matches your API response format exactly
- Has zero external dependencies
- Provides full TypeScript integration
- Handles authentication automatically
- Manages errors consistently

## Development Status

✅ **Completed:**
- Basic project setup with Vue 3 + Vuetify 3
- Routing configuration
- Basic page templates
- Custom HTTP client
- TypeScript configuration
- Docker integration

🚧 **In Progress:**
- Authentication store implementation
- API service integration
- Dynamic form components
- Data tables and CRUD operations

📋 **Planned:**
- Class definition management
- Dynamic entity CRUD
- API key management
- User permission system
- Charts and analytics 