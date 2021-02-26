package sqlite

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"net/url"
	"strings"

	sqlDriver "github.com/estuary/flow/go/materialize/driver/sql2"
	"github.com/estuary/flow/go/protocols/flow"
)

// NewSQLiteDriver creates a new Driver for sqlite.
func NewSQLiteDriver() *sqlDriver.Driver {
	return &sqlDriver.Driver{
		NewEndpoint: func(ctx context.Context, et flow.EndpointType, config json.RawMessage) (*sqlDriver.Endpoint, error) {
			var parsed struct {
				Path  string
				Table string
			}

			if err := json.Unmarshal(config, &parsed); err != nil {
				return nil, fmt.Errorf("parsing SQLite configuration: %w", err)
			}
			if parsed.Path == "" {
				return nil, fmt.Errorf("expected SQLite database configuration `path`")
			}
			if parsed.Table == "" {
				return nil, fmt.Errorf("expected SQLite database configuration `table`")
			}

			if u, err := url.Parse(parsed.Path); err != nil {
				return nil, fmt.Errorf("parsing path %q: %w", parsed.Path, err)
			} else if !u.IsAbs() {
				return nil, fmt.Errorf("path %q is not absolute", parsed.Path)
			} else if u.Scheme == "file" {
				// We can directly pass file:// schemes to SQLite.
			} else {
				// Path is absolute and non-local (e.x. https://some/database.db).
				// Mangle to turn into a file opened relative to the current directory.
				var parts = append([]string{u.Host}, strings.Split(u.Path, "/")...)
				parsed.Path = strings.Join(parts, "_")
			}

			db, err := sql.Open("sqlite3", parsed.Path)
			if err == nil {
				err = db.PingContext(ctx)
			}
			if err != nil {
				return nil, fmt.Errorf("opening SQLite database %q: %w", parsed.Path, err)
			}

			var endpoint = &sqlDriver.Endpoint{
				Context:      ctx,
				EndpointType: et,
				DB:           db,
				Generator:    sqlDriver.SQLiteSQLGenerator(),
			}
			endpoint.Tables.Target = parsed.Table
			endpoint.Tables.Checkpoints = sqlDriver.DefaultGazetteCheckpoints
			endpoint.Tables.Specs = sqlDriver.DefaultFlowMaterializations

			return endpoint, nil
		},
		RunTransactions: sqlDriver.RunSQLTransactions,
	}
}
