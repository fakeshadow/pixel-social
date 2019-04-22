import 'package:flutter/material.dart';
import 'package:pixel_flutter/components/Button/UserButton.dart';

const double NAV_BAR_MAX_HEIGHT = 300;

class SliverNavBarExpand extends StatelessWidget {
  Widget build(BuildContext context) {
    return SliverAppBar(
        elevation: 0,
        expandedHeight: NAV_BAR_MAX_HEIGHT,
        leading: Container(),
        floating: false,
        pinned: true,
        actions: <Widget>[UserButton()],
        flexibleSpace: LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) {
          double scale = (constraints.maxHeight - kToolbarHeight) / 268;
          return FlexibleSpaceBar(
              centerTitle: true,
              title: Text('REP: ' + '9000' + '  |  ' + 'PXS: ' + '9000',
                  style: TextStyle(
                    color: Colors.white,
                    fontSize: 13.0,
                  )),
              background: Container(
                  margin: EdgeInsets.all(0),
                  height: NAV_BAR_MAX_HEIGHT,
                  decoration: BoxDecoration(
                      image: DecorationImage(
                    fit: BoxFit.cover,
                    image: NetworkImage(
                        "https://images.pexels.com/photos/396547/pexels-photo-396547.jpeg?auto=compress&cs=tinysrgb&h=350"),
                  )),
                  child: Opacity(
                      opacity: scale >= 1 ? 1 : scale / 1.5,
                      child: Stack(children: [
                        Transform.scale(
                            scale: scale,
                            child: Stack(
                                alignment: Alignment.topCenter,
                                children: [
                                  Positioned(
                                    width: 100,
                                    height: 100,
                                    top: 100,
                                    child: CircleAvatar(
                                      backgroundImage:
                                          AssetImage('assets/category_default_cover.png'),
                                    ),
                                  ),
                                  Positioned(
                                      top: 220,
                                      child: Text('fakeshadow',
                                          style: TextStyle(
                                              fontSize: 20,
                                              color: Colors.white,
                                              fontWeight: FontWeight.bold))),
                                ]))
                      ]))));
        }));
  }
}
