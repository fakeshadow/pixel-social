import 'package:flutter/material.dart';
import '../components/NavigationBar/NavBarCommon.dart';
import '../components/NavigationBar/TabNavBar.dart';
import '../components/Categories/CategoryCard.dart';

class CommunityPage extends StatefulWidget {
  @override
  _CommunityPageState createState() => _CommunityPageState();
}

class _CommunityPageState extends State<CommunityPage> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
        bottomNavigationBar: TabNavBar(1),
        body: CustomScrollView(
          slivers: <Widget>[
            NavBarCommon(title: 'Community', isClose: false),
            SliverPadding(
                padding: const EdgeInsets.all(10.0),
                sliver: SliverList(
                  delegate: SliverChildListDelegate([
                    CategoryCard(categoryName: 'Games', categoryId: 1),
                    CategoryCard(categoryName: 'General', categoryId: 2),
                    CategoryCard(categoryName: 'Platform', categoryId: 3),
                    CategoryCard(categoryName: 'Group', categoryId: 4),
                    CategoryCard(categoryName: 'Other', categoryId: 5),
                    CategoryCard(categoryName: 'Announcement', categoryId: 6)
                  ]),
                ))
          ],
        ));
  }
}
