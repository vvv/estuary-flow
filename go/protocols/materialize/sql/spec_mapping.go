package sql

import (
	"fmt"
	"strings"

	pf "github.com/estuary/flow/go/protocols/flow"
)

// TableForMaterialization converts a MaterializationSpec into the Table representation that's used
// by Generator. This assumes that the MaterializationSpec has already been validated to
// ensure that each projection has exactly one type besides "null".
func TableForMaterialization(name string, comment string, identifierRenderer *Renderer, spec *pf.MaterializationSpec_Binding) *Table {
	return &Table{
		Name:       name,
		Identifier: identifierRenderer.Render(name),
		Comment:    comment,
		Columns:    columnsForMaterialization(spec, identifierRenderer),
	}
}

// Returns a slice of Columns for the materialization. This function always puts the root document
// projection at the end, so it's always at a known position for dealing with insert and update
// statements.
func columnsForMaterialization(spec *pf.MaterializationSpec_Binding, identifierRenderer *Renderer) []Column {
	var allFields = spec.FieldSelection.AllFields()
	var columns = make([]Column, 0, len(allFields))
	for _, field := range allFields {
		var projection = spec.Collection.GetProjection(field)
		columns = append(columns, ColumnForProjection(projection, identifierRenderer))
	}
	return columns
}

// ColumnForProjection returns a Column that is appropriate for storing values from the given Projection.
func ColumnForProjection(projection *pf.Projection, identifierRenderer *Renderer) Column {
	var column = Column{
		Name:       projection.Field,
		Identifier: identifierRenderer.Render(projection.Field),
		Comment:    commentForProjection(projection),
		PrimaryKey: projection.IsPrimaryKey,
		Type:       columnType(projection),
		NotNull:    projection.Inference.Exists == pf.Inference_MUST && !sliceContains("null", projection.Inference.Types),
	}
	if projection.Inference.String_ != nil {
		var s = projection.Inference.String_
		column.StringType = &StringTypeInfo{
			Format:      s.Format,
			ContentType: s.ContentType,
			MaxLength:   s.MaxLength,
		}
	}
	return column
}

func columnType(projection *pf.Projection) ColumnType {
	for _, ty := range projection.Inference.Types {
		switch ty {
		case "string":
			return STRING
		case "integer":
			return INTEGER
		case "number":
			return NUMBER
		case "boolean":
			return BOOLEAN
		case "object":
			return OBJECT
		case "array":
			return ARRAY
		}
	}
	panic("attempt to create column with no non-null type")
}

func commentForProjection(projection *pf.Projection) string {
	var source = "auto-generated"
	if projection.UserProvided {
		source = "user-provided"
	}
	var types = strings.Join(projection.Inference.Types, ", ")
	return fmt.Sprintf("%s projection of JSON at: %s with inferred types: [%s]", source, projection.Ptr, types)
}

func generateApplyStatements(
	endpoint Endpoint,
	existing map[string]*pf.MaterializationSpec_Binding,
	spec *pf.MaterializationSpec_Binding,
) ([]string, error) {
	var target = ResourcePath(spec.ResourcePath).Join()

	current, constraints, err := loadConstraints(target, spec.DeltaUpdates, &spec.Collection, existing)
	if err != nil {
		return nil, err
	}

	// Validate the request binding is a valid solution for its own constraints.
	if err = ValidateSelectedFields(constraints, spec); err != nil {
		return nil, fmt.Errorf("re-validating materialization: %w", err)
	}

	// We don't handle any form of schema migrations, so we require that the list of
	// fields in the request is identical to the current fields.
	if current != nil && !spec.FieldSelection.Equal(&current.FieldSelection) {
		return nil, fmt.Errorf(
			"the set of %s fields in the request differs from the existing fields,"+
				"which is disallowed because this driver does not perform schema migrations. "+
				"Request fields: %s , Existing fields: %s",
			target,
			spec.FieldSelection.String(),
			current.FieldSelection.String(),
		)
	}

	// If schema was already applied, there's no further work to be done.
	if current != nil {
		return nil, nil
	}

	// Like my grandpappy always told me, "never generate a SQL without a comment at the top"
	var comment = endpoint.Generator().CommentRenderer.Render(fmt.Sprintf(
		"Generated by Flow for materializing collection '%s'\nto table: %s",
		spec.Collection.Collection,
		target,
	))

	createSQL, err := endpoint.CreateTableStatement(
		TableForMaterialization(target, comment, endpoint.Generator().IdentifierRenderer, spec))
	if err != nil {
		return nil, fmt.Errorf("generating %s schema: %w", target, err)
	}

	return []string{createSQL}, nil
}
