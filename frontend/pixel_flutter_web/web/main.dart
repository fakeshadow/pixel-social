import 'package:flutter_web_ui/ui.dart' as ui;
import 'package:pixel_flutter_web/main.dart' as app;
import 'package:flutter_web_ui/src/engine.dart' as engine;

main() async {
  await ui.webOnlyInitializePlatform(
      assetManager: engine.AssetManager(
          assetsDir: 'assets'
      )
  );
  app.main();
}
