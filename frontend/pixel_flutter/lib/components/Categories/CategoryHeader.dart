import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/text.dart';

class CategoryHeader extends StatefulWidget {
  final int tabIndex;
  CategoryHeader({this.tabIndex});
  @override
  _CategoryHeaderState createState() => _CategoryHeaderState();
}

class _CategoryHeaderState extends State<CategoryHeader>
    with SingleTickerProviderStateMixin{

  final List _detail = [
    {'header': 'Hot', 'subHeader': 'What is popular today?'},
    {'header': 'Game', 'subHeader': 'Check out your favourite games'},
    {'header': 'Talk', 'subHeader': 'Communicating is most fun'},
  ];

  AnimationController _animationController;
  Animation<Offset> _animationOffset;

  initAnimation() {
    _animationController.reset();
    _animationController.forward();
  }

  @override
  void initState() {
    _animationController = AnimationController(
        vsync: this, duration: Duration(milliseconds: 300));
    _animationOffset =
        Tween<Offset>(begin:Offset(1,0), end:Offset(0,0)).animate(_animationController);
    super.initState();
  }

  @override
  void dispose() {
    _animationController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
      future: initAnimation(),
      builder: (context,snapshot) {
        return SlideTransition(
          position: _animationOffset,
          child: Padding(
            padding: EdgeInsets.symmetric(horizontal: 24.0),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  _detail[widget.tabIndex]['header'],
                  style: categoryHeaderStyle,
                ),
                Text(
                  _detail[widget.tabIndex]['subHeader'],
                  style: categorySubHeaderStyle,
                ),
              ],
            ),
          ),
        );
      }
    );
  }
}