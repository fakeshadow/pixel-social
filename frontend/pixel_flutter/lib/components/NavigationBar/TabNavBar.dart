import 'package:flutter/material.dart';

class TabNavBar extends StatefulWidget {
  final int currentPage;
  TabNavBar(this.currentPage);

  @override
  _TabNavBarState createState() => _TabNavBarState();
}

class _TabNavBarState extends State<TabNavBar> {
  @override
  Widget build(BuildContext context) {
    return BottomNavigationBar(
      currentIndex: widget.currentPage,
      onTap: (int index) {
        if (index == 0) {
          Navigator.of(context).pushNamed('/');
        } else if (index == 1) {
          Navigator.of(context).pushNamed('/community');
        } else if (index == 2) {
          Navigator.of(context).pushNamed('/profile');
        }
      }, // this will be set when a new tab is tapped
      items: [
        BottomNavigationBarItem(
          icon: new Icon(Icons.home),
          title: new Text('Home'),
        ),
        BottomNavigationBarItem(
          icon: new Icon(Icons.people),
          title: new Text('Community'),
        ),
        BottomNavigationBarItem(
            icon: Icon(Icons.person), title: Text('Profile'))
      ],
    );
  }
}
