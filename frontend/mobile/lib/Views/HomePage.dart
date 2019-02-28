import 'package:flutter/material.dart';
import '../components/NavigationBar/NavBarCommon.dart';
import '../components/NavigationBar/TabNavBar.dart';

class HomePage extends StatefulWidget {
  @override
  _HomePageState createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  bool isUnread = true;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
        bottomNavigationBar: TabNavBar(0),
        body: CustomScrollView(
          slivers: <Widget>[
            NavBarCommon(title: 'Home', isClose: false),
            SliverList(
              delegate: SliverChildListDelegate(buildTextViews(50)),
            ),
          ],
        ));
  }
}

List buildTextViews(int count) {
  List<Widget> strings = List();
  for (int i = 0; i < count; i++) {
    strings.add(new Padding(
        padding: new EdgeInsets.all(16.0),
        child: new Text("Item number " + i.toString(),
            style: new TextStyle(fontSize: 20.0))));
  }
  return strings;
}
