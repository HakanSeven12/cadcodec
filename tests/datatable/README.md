# Native DATATABLE Fixtures

`point_object_id.dwg` and `point_object_id.dxf` were generated in AutoCAD
2027 using its .NET API, with no customer drawing content. Both use AC1032.

A new database contains a DATATABLE under the named-object dictionary key
`Review93DATATABLE`. It has one row and two columns:

| Column | Native CellType | Value |
| --- | --- | --- |
| Point | Point (4) | (1.5, 2.5, 3.5) |
| ObjectId | ObjectId (5) | Layer table, handle 2 |

The DXF encodes the point using groups 10/20/30 and the object ID using 331.
These fixtures guard against matching mistakes in the library's own reader
and writer. They were generated with `DataTable.AppendColumn`,
`DataCell.SetPoint`, `DataCell.SetObjectId`, `DataTable.AppendRow`,
`Database.SaveAs`, and `Database.DxfOut`.
