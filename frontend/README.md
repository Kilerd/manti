# Manti Gateway Frontend

A modern React-based user interface for the Manti LLM Gateway, built with Vite and styled with Tailwind CSS + Shadcn UI components.

## Features

- **User Authentication**: Secure login and registration system
- **Dashboard**: Overview of API usage statistics and costs
- **API Key Management**: Create, view, and revoke API keys
- **Usage History**: Detailed tracking of API requests and token consumption
- **Profile Management**: Update user profile and change password
- **Responsive Design**: Mobile-friendly interface with collapsible sidebar

## Tech Stack

- **React 18** - UI framework
- **Vite** - Build tool and dev server
- **React Router** - Client-side routing
- **Tailwind CSS** - Utility-first CSS framework
- **Shadcn UI** - Beautifully designed components
- **Axios** - HTTP client for API communication
- **Lucide React** - Icon library

## Getting Started

### Prerequisites

- Node.js 16+ installed
- Manti backend running on http://localhost:8080

### Installation

1. Install dependencies:
```bash
npm install
```

2. Configure the API endpoint (optional):
Create a `.env` file if you need to use a different backend URL:
```
VITE_API_BASE_URL=http://your-backend-url:8080
```

3. Start the development server:
```bash
npm run dev
```

The application will be available at http://localhost:5173 (or next available port).

### Production Build

To build for production:
```bash
npm run build
```

The built files will be in the `dist` directory.

To preview the production build:
```bash
npm run preview
```

## Project Structure

```
src/
├── components/
│   ├── ui/           # Shadcn UI components
│   │   ├── button.jsx
│   │   ├── card.jsx
│   │   ├── input.jsx
│   │   ├── label.jsx
│   │   └── table.jsx
│   └── Layout.jsx    # Main layout with sidebar
├── pages/
│   ├── Login.jsx     # Login page
│   ├── Register.jsx  # Registration page
│   ├── Dashboard.jsx # Dashboard with statistics
│   ├── ApiKeys.jsx   # API key management
│   ├── Usage.jsx     # Usage history
│   └── Profile.jsx   # User profile settings
├── services/
│   └── api.js        # API service layer
├── lib/
│   └── utils.js      # Utility functions
├── App.jsx           # Main app component with routing
├── main.jsx          # Application entry point
└── index.css         # Global styles with Tailwind
```

## Available Pages

1. **Login** (`/login`) - User authentication
2. **Register** (`/register`) - New user registration
3. **Dashboard** (`/dashboard`) - Overview of usage and costs
4. **API Keys** (`/api-keys`) - Manage API keys for accessing the gateway
5. **Usage** (`/usage`) - Detailed usage history and statistics
6. **Profile** (`/profile`) - User profile and password management

## API Integration

The frontend communicates with the Manti backend API endpoints:

- `POST /auth/login` - User login
- `POST /auth/register` - User registration
- `GET /api-keys` - List API keys
- `POST /api-keys` - Create new API key
- `DELETE /api-keys/:id` - Revoke API key
- `GET /usage/stats` - Get usage statistics
- `GET /usage/history` - Get usage history
- `GET /user/profile` - Get user profile
- `PUT /user/profile` - Update user profile

## Development

### Adding New Components

Shadcn UI components can be added manually by creating them in `src/components/ui/`. The project is set up to use the custom color scheme defined in `src/index.css`.

### Customizing Theme

Theme colors and styles can be customized in:
- `src/index.css` - CSS variables for colors
- `tailwind.config.js` - Tailwind configuration

## License

MIT