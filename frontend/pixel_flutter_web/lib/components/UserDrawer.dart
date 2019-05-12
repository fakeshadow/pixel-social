import 'package:flutter_web/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/blocs/UserBlocs.dart';

import 'package:pixel_flutter_web/style/colors.dart';
import 'package:pixel_flutter_web/style/text.dart';

class UserDrawer extends StatefulWidget {
  @override
  _UserDrawerState createState() => _UserDrawerState();
}

class _UserDrawerState extends State<UserDrawer> {
  List<DrawerItem> drawerItems = [
    DrawerItem(title: 'Message', icon: Icons.message),
    DrawerItem(title: 'Setting', icon: Icons.settings),
    DrawerItem(title: 'Wallet', icon: Icons.account_balance_wallet),
    DrawerItem(title: 'Logout', icon: Icons.exit_to_app),
  ];

  void _handleDrawerItem(int index) {
    if (index == 0) {
      Navigator.pop(context);
    }
    if (index == 1) {
      Navigator.popAndPushNamed(context, 'profile');
    }
    if (index == 2) {
      print(2);
    }
    if (index == 3) {
      BlocProvider.of<UserBloc>(context).dispatch(LoggingOut());
      Navigator.pop(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      color: primaryColor.withOpacity(0.9),
      width: 150.0,
      child: Column(
        children: <Widget>[
          SizedBox(
            height: 70,
          ),
          Expanded(
            child: ListView.builder(
              itemBuilder: (context, index) {
                return Padding(
                  padding: EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                  child: CollapsingListTile(
                      title: drawerItems[index].title,
                      icon: drawerItems[index].icon,
                      callback: () => _handleDrawerItem(index)),
                );
              },
              itemCount: drawerItems.length,
            ),
          )
        ],
      ),
    );
  }
}

class DrawerItem {
  String title;
  IconData icon;

  DrawerItem({this.title, this.icon});
}

class CollapsingListTile extends StatefulWidget {
  final String title;
  final IconData icon;
  final Function callback;

  CollapsingListTile(
      {@required this.title, @required this.icon, @required this.callback});

  @override
  _CollapsingListTileState createState() => _CollapsingListTileState();
}

class _CollapsingListTileState extends State<CollapsingListTile> {
  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: () => widget.callback(),
      child: Row(
        children: <Widget>[
          Icon(
            widget.icon,
            color: Colors.white30,
            size: 22,
          ),
          SizedBox(
            width: 10,
          ),
          Text(
            widget.title,
            style: drawerTextStyle,
          )
        ],
      ),
    );
  }
}