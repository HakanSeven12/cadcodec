using System;
using System.IO;
using Autodesk.AutoCAD.DatabaseServices;
using Autodesk.AutoCAD.Geometry;
using Autodesk.AutoCAD.Runtime;

[assembly: CommandClass(typeof(ReadTruth.BlockNamesFixtures))]
namespace ReadTruth {
    public class BlockNamesFixtures {
        [CommandMethod("MAKEBLOCKNAMESFIXTURE", CommandFlags.Session)]
        public static void Run() {
            var output = Environment.GetEnvironmentVariable("READTRUTH_OUT");
            Directory.CreateDirectory(Path.GetDirectoryName(output));
            var path = Path.Combine(Path.GetDirectoryName(output), "anonymous-names.dwg");
            using var db = new Database(true, true);
            using (var tr = db.TransactionManager.StartTransaction()) {
                var blocks = (BlockTable)tr.GetObject(db.BlockTableId, OpenMode.ForWrite);
                var model = (BlockTableRecord)tr.GetObject(blocks[BlockTableRecord.ModelSpace], OpenMode.ForWrite);
                foreach (var name in new[] { "NamedBefore", "*U", "NamedBetween", "*U", "*U" }) {
                    var block = new BlockTableRecord { Name = name };
                    blocks.Add(block); tr.AddNewlyCreatedDBObject(block, true);
                    var definition = new AttributeDefinition(Point3d.Origin, "value", "TAG", "Prompt", db.Textstyle);
                    block.AppendEntity(definition); tr.AddNewlyCreatedDBObject(definition, true);
                    var insert = new BlockReference(Point3d.Origin, block.ObjectId);
                    model.AppendEntity(insert); tr.AddNewlyCreatedDBObject(insert, true);
                    var attribute = new AttributeReference();
                    attribute.SetAttributeFromBlock(definition, insert.BlockTransform);
                    insert.AttributeCollection.AppendAttribute(attribute); tr.AddNewlyCreatedDBObject(attribute, true);
                }
                tr.Commit();
            }
            db.SaveAs(path, DwgVersion.Current);
            File.WriteAllLines(output, new[] { path });
        }
    }
}
